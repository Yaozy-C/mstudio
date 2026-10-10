use super::{ModelOutput, ProviderAdapter, ProviderFuture};
use crate::jobs::client;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
pub struct HttpJson;
#[path = "http_headers.rs"]
mod headers;
pub use headers::header_pairs;
use headers::with_headers;
pub fn endpoint(value: &str) -> Result<reqwest::Url> {
    let url = reqwest::Url::parse(value)?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    ensure!(
        url.scheme() == "https" || (url.scheme() == "http" && local),
        "使用 HTTPS 或本机 HTTP 地址"
    );
    ensure!(
        url.username().is_empty() && url.password().is_none() && url.fragment().is_none(),
        "服务地址不能包含凭据或片段"
    );
    Ok(url)
}
pub fn validate(config: &Value) -> Result<()> {
    ensure!(config.is_object(), "请配置 HTTP 结果映射");
    for field in [
        "outputPointer",
        "idPointer",
        "statusPointer",
        "itemUrlPointer",
    ] {
        if let Some(value) = config.get(field) {
            ensure!(
                value
                    .as_str()
                    .is_some_and(|s| s.is_empty() || s.starts_with('/')),
                "{field} 必须使用 JSON Pointer，例如 /data/0/url"
            );
        }
    }
    ensure!(
        config["outputPointer"].is_string(),
        "请填写结果 URL 的 JSON Pointer"
    );
    if config
        .get("pollUrl")
        .is_some_and(|v| v.as_str().is_some_and(|s| !s.is_empty()))
    {
        ensure!(
            config["idPointer"].is_string()
                && config["statusPointer"].is_string()
                && config["doneValue"].is_string(),
            "异步接口需要任务 ID、状态字段和完成状态"
        );
    }
    header_pairs(config)?;
    Ok(())
}
fn request(key: &str, url: &str) -> Result<reqwest::RequestBuilder> {
    endpoint(url)?;
    let request = client()?.get(url);
    Ok(if key.is_empty() {
        request
    } else {
        request.bearer_auth(key)
    })
}
async fn response(mut response: reqwest::Response, stage: &str) -> Result<Value> {
    if !response.status().is_success() {
        return Err(crate::app_error::http_error(response, stage).await.into());
    }
    ensure!(
        response.content_length().unwrap_or(0) <= 8_000_000,
        "服务响应过大"
    );
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(bytes.len() + chunk.len() <= 8_000_000, "服务响应过大");
        bytes.extend_from_slice(&chunk);
    }
    Ok(serde_json::from_slice(&bytes)?)
}
fn mapped_outputs(value: &Value, config: &Value) -> Result<Vec<ModelOutput>> {
    let pointer = config["outputPointer"].as_str().context("缺少结果字段")?;
    let target = value
        .pointer(pointer)
        .context("响应中未找到结果 URL，请检查结果字段映射")?;
    let values = target
        .as_array()
        .cloned()
        .unwrap_or_else(|| vec![target.clone()]);
    let kind = config["kind"].as_str().unwrap_or("image");
    let mut outputs = Vec::new();
    for item in values {
        let url =
            if let Some(field) = config["itemUrlPointer"].as_str().filter(|s| !s.is_empty()) {
                item.pointer(field).and_then(Value::as_str)
            } else {
                item.as_str()
            }
            .context("结果必须是 URL 或 URL 数组，请检查映射")?;
        endpoint(url)?;
        outputs.push(ModelOutput {
            kind: kind.into(),
            url: url.into(),
        });
    }
    ensure!(!outputs.is_empty(), "生成接口未返回媒体结果");
    Ok(outputs)
}
fn complete(value: &Value, config: &Value) -> Result<super::ProviderUpdate> {
    let outputs = mapped_outputs(value, config)?;
    Ok(serde_json::from_value(
        json!({"status":"COMPLETED","outputs":outputs,"result":{"outputs":outputs}}),
    )?)
}
pub(super) fn poll_url(base: &str, template: &str, id: &str) -> Result<String> {
    let escaped: String = id
        .bytes()
        .map(|b| {
            if b.is_ascii_alphanumeric() || b"-._~".contains(&b) {
                (b as char).to_string()
            } else {
                format!("%{b:02X}")
            }
        })
        .collect();
    let url = endpoint(base)?.join(&template.replace("{id}", &escaped))?;
    let base = endpoint(base)?;
    ensure!(
        url.origin() == base.origin() && url.username().is_empty() && url.password().is_none(),
        "轮询地址必须与提交服务同源"
    );
    Ok(url.to_string())
}
impl ProviderAdapter for HttpJson {
    fn id(&self) -> &'static str {
        "http-json"
    }
    fn validate_endpoint(&self, value: &str) -> Result<()> {
        endpoint(value).map(|_| ())
    }
    fn submit<'a>(
        &'a self,
        key: &'a str,
        endpoint_url: &'a str,
        input: &'a Value,
        config: &'a Value,
        storage_lease: super::StorageLease,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            let _storage_lease = storage_lease;
            validate(config)?;
            for field in ["pollUrl", "resultUrl"] {
                if let Some(template) = config[field].as_str().filter(|s| !s.is_empty()) {
                    poll_url(endpoint_url, template, "validation")?;
                }
            }
            let mut builder = client()?.post(endpoint(endpoint_url)?).json(input);
            if !key.is_empty() {
                builder = builder.bearer_auth(key);
            }
            let value = response(
                with_headers(builder, config)?
                    .send()
                    .await
                    .map_err(|_| anyhow::anyhow!("提交中断，结果未知；请核查服务后再重试"))?,
                "submit",
            )
            .await?;
            if let Some(template) = config["pollUrl"].as_str().filter(|s| !s.is_empty()) {
                let id = value
                    .pointer(config["idPointer"].as_str().unwrap())
                    .and_then(Value::as_str)
                    .context("服务未返回字符串任务 ID")?;
                let url = poll_url(endpoint_url, template, id)?;
                Ok(serde_json::from_value(
                    json!({"status":"IN_QUEUE","requestId":id,"statusUrl":url}),
                )?)
            } else {
                complete(&value, config)
            }
        })
    }
    fn refresh<'a>(&'a self, key: &'a str, job: &'a Value) -> ProviderFuture<'a> {
        Box::pin(async move {
            if job["status"] == "COMPLETED" {
                return Ok(serde_json::from_value(json!({"status":"COMPLETED"}))?);
            }
            let config = &job["providerConfig"];
            validate(config)?;
            let url = poll_url(
                job["endpoint"].as_str().context("缺少端点")?,
                config["pollUrl"].as_str().context("此接口没有轮询地址")?,
                job["requestId"]
                    .as_str()
                    .context("任务未取得 ID，请在服务端核查")?,
            )?;
            let value = response(
                with_headers(request(key, &url)?, config)?.send().await?,
                "status",
            )
            .await?;
            let status = value
                .pointer(config["statusPointer"].as_str().context("缺少状态映射")?)
                .and_then(Value::as_str)
                .context("未找到字符串任务状态")?;
            if Some(status) == config["doneValue"].as_str() {
                if let Some(template) = config["resultUrl"].as_str().filter(|s| !s.is_empty()) {
                    let url = poll_url(
                        job["endpoint"].as_str().unwrap(),
                        template,
                        job["requestId"].as_str().unwrap(),
                    )?;
                    complete(
                        &response(
                            with_headers(request(key, &url)?, config)?.send().await?,
                            "status",
                        )
                        .await?,
                        config,
                    )
                } else {
                    complete(&value, config)
                }
            } else if config["failedValues"]
                .as_array()
                .is_some_and(|v| v.iter().any(|v| v.as_str() == Some(status)))
            {
                Ok(serde_json::from_value(
                    json!({"status":"FAILED","error":"模型任务失败，请查看生成服务"}),
                )?)
            } else {
                Ok(serde_json::from_value(json!({"status":"IN_PROGRESS"}))?)
            }
        })
    }
    fn cancel<'a>(&'a self, _: &'a str, _: &'a Value) -> ProviderFuture<'a> {
        Box::pin(async {
            anyhow::bail!("此自定义接口未配置取消操作，请在服务端取消任务")
        })
    }
    fn outputs(&self, result: &Value) -> Vec<ModelOutput> {
        serde_json::from_value(result["outputs"].clone()).unwrap_or_default()
    }
}

#[cfg(test)]
#[path = "http_json_tests.rs"]
mod tests;
