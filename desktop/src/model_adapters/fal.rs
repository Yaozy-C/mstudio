use super::{ProviderAdapter, ProviderFuture};
use crate::jobs::client;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

pub struct Fal;
pub fn queue_url(url: &str) -> Result<String> {
    let parsed = reqwest::Url::parse(url)?;
    ensure!(
        parsed.scheme() == "https"
            && parsed.host_str() == Some("queue.fal.run")
            && parsed.username().is_empty()
            && parsed.password().is_none()
            && parsed.port().is_none(),
        "任务地址必须来自 fal 队列"
    );
    Ok(url.into())
}
async fn json_response(response: reqwest::Response, stage: &str) -> Result<Value> {
    if !response.status().is_success() {
        return Err(crate::app_error::http_error(response, stage).await.into());
    }
    Ok(response.json().await?)
}

impl ProviderAdapter for Fal {
    fn validate_endpoint(&self, endpoint: &str) -> Result<()> {
        ensure!(
            endpoint.len() <= 200
                && endpoint.split('/').count() >= 2
                && endpoint.split('/').all(|s| !s.is_empty()
                    && s.chars()
                        .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == '.')
                    && !s.starts_with('.')),
            "请填写 fal 模型的完整端点 ID"
        );
        Ok(())
    }
    fn id(&self) -> &'static str {
        "fal"
    }
    fn submit<'a>(
        &'a self,
        key: &'a str,
        endpoint: &'a str,
        input: &'a Value,
        _config: &'a Value,
        storage_lease: super::StorageLease,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            let _storage_lease = storage_lease;
            let response = client()?
                .post(format!("https://queue.fal.run/{endpoint}"))
                .header("Authorization", format!("Key {key}"))
                .json(input)
                .send()
                .await
                .map_err(|_| {
                    anyhow::anyhow!("提交连接中断；结果未知，请在服务商控制台核查，避免重复提交")
                })?;
            let value = json_response(response, "submit").await?;
            Ok(serde_json::from_value(
                json!({"requestId":value["request_id"],
                "statusUrl":queue_url(value["status_url"].as_str().context("缺少状态 URL")?)?,
                "responseUrl":queue_url(value["response_url"].as_str().context("缺少结果 URL")?)?,
                "cancelUrl":queue_url(value["cancel_url"].as_str().context("缺少取消 URL")?)?,
                "status":"IN_QUEUE"}),
            )?)
        })
    }
    fn refresh<'a>(&'a self, key: &'a str, job: &'a Value) -> ProviderFuture<'a> {
        Box::pin(async move {
            let url = queue_url(
                job["statusUrl"]
                    .as_str()
                    .context("提交结果未知，请先在服务商控制台核查")?,
            )?;
            let response = match json_response(
                client()?
                    .get(url)
                    .header("Authorization", format!("Key {key}"))
                    .send()
                    .await?,
                "status",
            )
            .await
            {
                Err(error)
                    if error
                        .downcast_ref::<crate::app_error::AppError>()
                        .is_some_and(|e| e.outcome.as_deref() == Some("cancelled")) =>
                {
                    return completed_error(
                        error.downcast::<crate::app_error::AppError>().unwrap(),
                    );
                }
                result => result?,
            };
            let mut update = json!({"status":response["status"]});
            if response["status"] == "COMPLETED" {
                if !response["error"].is_null() || !response["error_type"].is_null() {
                    let cancelled = response["error_type"] == "client_cancelled";
                    let mut error = crate::app_error::AppError::new(
                        if cancelled {
                            "JOB_CANCELLED"
                        } else {
                            "GENERATION_FAILED"
                        },
                        "result",
                        &crate::app_error::provider_details(&response),
                    );
                    error.outcome = Some(if cancelled { "cancelled" } else { "failed" }.into());
                    update = json!({"status":if cancelled { "CANCELLED" } else { "FAILED" },"error":error.to_string()});
                } else {
                    let url = queue_url(job["responseUrl"].as_str().context("缺少结果 URL")?)?;
                    let response = client()?
                        .get(url)
                        .header("Authorization", format!("Key {key}"))
                        .send()
                        .await?;
                    // A completed request can return validation failure at its result URL.
                    if !response.status().is_success() {
                        let error = crate::app_error::http_error(response, "result").await;
                        return completed_error(error);
                    }
                    let result = json_response(response, "result").await?;
                    update["outputs"] = serde_json::to_value(normalize_outputs(&result))?;
                    update["result"] = result;
                }
            }
            Ok(serde_json::from_value(update)?)
        })
    }
    fn cancel<'a>(&'a self, key: &'a str, job: &'a Value) -> ProviderFuture<'a> {
        Box::pin(async move {
            let url = queue_url(job["cancelUrl"].as_str().context("没有取消地址")?)?;
            let response = client()?
                .put(url)
                .header("Authorization", format!("Key {key}"))
                .send()
                .await?;
            if !response.status().is_success() {
                return Err(crate::app_error::http_error(response, "cancel")
                    .await
                    .into());
            }
            Ok(serde_json::from_value(
                json!({"status":"CANCEL_REQUESTED"}),
            )?)
        })
    }
    fn outputs(&self, value: &Value) -> Vec<super::ModelOutput> {
        normalize_outputs(value)
    }
}
fn normalize_outputs(value: &Value) -> Vec<super::ModelOutput> {
    let mut outputs = Vec::new();
    for kind in ["video", "audio", "image"] {
        if let Some(url) = value[kind]["url"].as_str() {
            outputs.push(super::ModelOutput {
                kind: kind.into(),
                url: url.into(),
            });
        }
    }
    if let Some(images) = value["images"].as_array() {
        for image in images {
            if let Some(url) = image["url"].as_str() {
                outputs.push(super::ModelOutput {
                    kind: "image".into(),
                    url: url.into(),
                });
            }
        }
    }
    outputs
}

fn completed_error(mut error: crate::app_error::AppError) -> Result<super::ProviderUpdate> {
    let status = if error.outcome.as_deref() == Some("cancelled") {
        "CANCELLED"
    } else if matches!(error.http_status, Some(400 | 422)) {
        error.outcome = Some("failed".into());
        "FAILED"
    } else {
        return Err(error.into());
    };
    Ok(serde_json::from_value(
        json!({"status":status,"error":error.to_string()}),
    )?)
}
#[cfg(test)]
mod recovery_tests {
    use super::*;
    use crate::app_error::AppError;
    #[test]
    fn completed_validation_failure_becomes_terminal_but_auth_and_network_do_not() {
        let update =
            completed_error(AppError::http(422, "result", "body.duration: invalid")).unwrap();
        assert_eq!(update.status, "FAILED");
        let issue: AppError = serde_json::from_str(&update.error.unwrap()).unwrap();
        assert_eq!(issue.code, "INVALID_INPUT");
        assert!(issue.details.contains("duration"));
        for status in [401, 403, 404, 429, 503] {
            assert!(completed_error(AppError::http(status, "result", "error")).is_err());
        }
        let mut issue = AppError::http(499, "result", "cancelled");
        issue.outcome = Some("cancelled".into());
        assert_eq!(completed_error(issue).unwrap().status, "CANCELLED");
    }
}
