//! DashScope asynchronous transport. Model IDs, controls and reference fields are data.
use super::{ModelOutput, ProviderAdapter, ProviderFuture, ProviderUpdate};
use crate::jobs::client;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

pub struct DashScope;
fn configuration() -> Result<Value> {
    Ok(serde_json::from_str(include_str!(
        "../../../frontend/src/models/config/dashscope.json"
    ))?)
}
pub(super) fn endpoint(value: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(value)?;
    ensure!(
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none()
            && url.host_str().is_some_and(|h| h.ends_with(".aliyuncs.com")),
        "请使用百炼官方 HTTPS 服务地址"
    );
    ensure!(
        !url.host_str().unwrap().starts_with("your-workspace-id."),
        "请在服务连接填写真实的百炼业务空间地址"
    );
    Ok(url)
}
pub fn submission_url(base: &str) -> Result<String> {
    // An unconfigured connection may be saved before the user supplies its workspace ID.
    let base = reqwest::Url::parse(base)?;
    Ok(base
        .join(
            configuration()?["submitPath"]
                .as_str()
                .context("缺少提交路径")?,
        )?
        .to_string())
}
fn task_url(base: &str, id: &str) -> Result<String> {
    ensure!(
        !id.is_empty()
            && id
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"-._".contains(&b)),
        "任务 ID 无效"
    );
    Ok(endpoint(base)?
        .join(&format!(
            "{}{id}",
            configuration()?["taskPath"]
                .as_str()
                .context("缺少任务路径")?
        ))?
        .to_string())
}
pub(super) async fn response(response: reqwest::Response, stage: &str) -> Result<Value> {
    if !response.status().is_success() {
        return Err(crate::app_error::http_error(response, stage).await.into());
    }
    let value: Value = response.json().await?;
    if value["code"].as_str().is_some() {
        let mut issue = crate::app_error::AppError::new(
            "GENERATION_FAILED",
            stage,
            &crate::app_error::provider_details(&value),
        );
        issue.outcome = Some("rejected".into());
        return Err(issue.into());
    }
    Ok(value)
}
fn body(input: &Value) -> Result<Value> {
    let mut body = input.clone();
    let object = body.as_object_mut().context("参数须为 JSON 对象")?;
    let prompt = object.remove("prompt");
    let mut media = Vec::new();
    for (field, kind) in configuration()?["referenceTypes"]
        .as_object()
        .context("参考映射无效")?
    {
        if let Some(value) = object.remove(field) {
            for url in value.as_array().cloned().unwrap_or_else(|| vec![value]) {
                media.push(json!({"type":kind,"url":url}));
            }
        }
    }
    if body["input"].is_null() {
        body["input"] = json!({});
    }
    if let Some(prompt) = prompt {
        body["input"]["prompt"] = prompt;
    }
    if !media.is_empty() {
        body["input"]["media"] = json!(media);
    }
    Ok(body)
}
fn update(value: Value) -> Result<ProviderUpdate> {
    let status = match value["output"]["task_status"]
        .as_str()
        .context("服务未返回任务状态")?
    {
        "PENDING" => "IN_QUEUE",
        "RUNNING" => "IN_PROGRESS",
        "SUCCEEDED" => "COMPLETED",
        "FAILED" => "FAILED",
        "UNKNOWN" => "FAILED",
        "CANCELED" => "CANCELLED",
        other => anyhow::bail!("未知百炼任务状态：{other}"),
    };
    let mut result = json!({"status":status});
    if status == "COMPLETED" {
        let outputs = outputs(&value)?;
        result["outputs"] = serde_json::to_value(outputs)?;
        result["result"] = value.clone();
    } else if value["output"]["task_status"] == "UNKNOWN" {
        let mut error = crate::app_error::AppError::new(
            "RESULT_UNAVAILABLE",
            "status",
            "百炼任务不存在或已超过查询保留期，无法获取生成结果",
        );
        error.outcome = Some("failed".into());
        result["error"] = json!(error.to_string());
    } else if status == "FAILED" {
        result["error"] = json!(crate::app_error::provider_details(&value["output"]));
    }
    if matches!(status, "FAILED" | "CANCELLED")
        && value["output"]
            .get("video_url")
            .is_some_and(|url| !url.is_null())
    {
        result["outputs"] = serde_json::to_value(outputs(&value)?)?;
        result["result"] = value;
    }
    let update: ProviderUpdate = serde_json::from_value(result)?;
    update.validate()?;
    Ok(update)
}
pub(super) fn outputs(value: &Value) -> Result<Vec<ModelOutput>> {
    let url = value["output"]["video_url"]
        .as_str()
        .filter(|url| !url.trim().is_empty())
        .context("百炼结果格式无效：output.video_url 须为非空字符串")?;
    Ok(vec![ModelOutput {
        kind: "video".into(),
        url: url.into(),
    }])
}
impl ProviderAdapter for DashScope {
    fn id(&self) -> &'static str {
        "dashscope"
    }
    fn validate_endpoint(&self, value: &str) -> Result<()> {
        // Permit the preset placeholder while configuring, but never send to it.
        let url = reqwest::Url::parse(value)?;
        ensure!(
            url.scheme() == "https"
                && url.username().is_empty()
                && url.password().is_none()
                && url.query().is_none()
                && url.fragment().is_none()
                && url.host_str().is_some_and(|h| h.ends_with(".aliyuncs.com")),
            "请使用百炼官方 HTTPS 服务地址"
        );
        Ok(())
    }
    fn submit<'a>(
        &'a self,
        key: &'a str,
        endpoint_url: &'a str,
        input: &'a Value,
        _: &'a Value,
        lease: super::StorageLease,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            let _lease = lease;
            ensure!(!key.is_empty(), "请在服务连接填写百炼 API Key");
            let value = response(
                client()?
                    .post(endpoint(endpoint_url)?)
                    .bearer_auth(key)
                    .header("X-DashScope-Async", "enable")
                    .header("X-DashScope-OssResourceResolve", "enable")
                    .json(&body(input)?)
                    .send()
                    .await?,
                "submit",
            )
            .await?;
            let id = value["output"]["task_id"]
                .as_str()
                .context("服务未返回任务 ID")?;
            let mut update = update(value.clone())?;
            update.request_id = Some(id.into());
            update.status_url = Some(task_url(endpoint_url, id)?);
            Ok(update)
        })
    }
    fn refresh<'a>(&'a self, key: &'a str, job: &'a Value) -> ProviderFuture<'a> {
        Box::pin(async move {
            let url = task_url(
                job["endpoint"].as_str().context("缺少端点")?,
                job["requestId"]
                    .as_str()
                    .context("生成任务缺少百炼任务 ID")?,
            )?;
            update(response(client()?.get(url).bearer_auth(key).send().await?, "status").await?)
        })
    }
    fn cancel<'a>(&'a self, key: &'a str, job: &'a Value) -> ProviderFuture<'a> {
        Box::pin(async move {
            let url = task_url(
                job["endpoint"].as_str().context("缺少端点")?,
                job["requestId"].as_str().context("缺少任务 ID")?,
            )?;
            response(
                client()?
                    .post(format!("{url}/cancel"))
                    .bearer_auth(key)
                    .send()
                    .await?,
                "cancel",
            )
            .await?;
            Ok(serde_json::from_value(
                json!({"status":"CANCEL_REQUESTED"}),
            )?)
        })
    }
}
#[path = "dashscope_upload.rs"]
mod storage;
pub use storage::upload;

#[cfg(test)]
mod status_tests {
    use super::*;
    #[test]
    fn unavailable_provider_task_finishes_as_failed() {
        let state = update(json!({"output":{"task_status":"UNKNOWN"}})).unwrap();
        assert_eq!(state.status, "FAILED");
        let error: crate::app_error::AppError =
            serde_json::from_str(&state.error.unwrap()).unwrap();
        assert_eq!(error.code, "RESULT_UNAVAILABLE");
        assert_eq!(error.outcome.as_deref(), Some("failed"));
        assert!(!error.retryable);
    }
    #[test]
    fn completion_and_failed_partial_video_have_actual_top_level_count() {
        for (raw, status) in [
            ("SUCCEEDED", "COMPLETED"),
            ("FAILED", "FAILED"),
            ("CANCELED", "CANCELLED"),
        ] {
            let mut job = json!({});
            update(
                json!({"output":{"task_status":raw,"video_url":"https://example.com/video.mp4"}}),
            )
            .unwrap()
            .apply(&mut job)
            .unwrap();
            assert_eq!(job["status"], status);
            assert_eq!(job["outputCount"], 1);
            assert_eq!(job["outputs"][0]["kind"], "video");
        }
        assert!(update(json!({"output":{"task_status":"SUCCEEDED"}})).is_err());
    }
}
