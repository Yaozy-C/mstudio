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
        "FAILED" | "UNKNOWN" => "FAILED",
        "CANCELED" => "CANCELLED",
        other => anyhow::bail!("未知百炼任务状态：{other}"),
    };
    let mut result = json!({"status":status});
    if status == "COMPLETED" {
        let outputs = outputs(&value);
        ensure!(!outputs.is_empty(), "生成成功但未返回视频地址");
        result["outputs"] = serde_json::to_value(outputs)?;
        result["result"] = value;
    } else if status == "FAILED" {
        result["error"] = json!(crate::app_error::provider_details(&value["output"]));
    }
    Ok(serde_json::from_value(result)?)
}
fn outputs(value: &Value) -> Vec<ModelOutput> {
    value["output"]["video_url"]
        .as_str()
        .map(|url| {
            vec![ModelOutput {
                kind: "video".into(),
                url: url.into(),
            }]
        })
        .unwrap_or_default()
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
                    .await
                    .map_err(|_| {
                        anyhow::anyhow!("提交连接中断，结果未知；请在百炼控制台核查后再重试")
                    })?,
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
                    .context("提交结果未知，请在百炼控制台核查")?,
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
    fn outputs(&self, value: &Value) -> Vec<ModelOutput> {
        outputs(value)
    }
}
#[path = "dashscope_upload.rs"]
mod storage;
pub use storage::upload;
