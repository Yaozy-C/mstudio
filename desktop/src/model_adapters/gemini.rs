use super::image_data::image_bytes;
use super::{ModelOutput, ProviderAdapter, ProviderFuture};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
pub struct Gemini;
pub fn endpoint(value: &str) -> Result<()> {
    ensure!(
        matches!(
            value.trim_end_matches('/'),
            "https://generativelanguage.googleapis.com"
                | "https://generativelanguage.googleapis.com/v1beta"
                | "https://generativelanguage.googleapis.com/v1"
        ),
        "Google 官方连接请使用 generativelanguage.googleapis.com"
    );
    Ok(())
}
fn payload(input: &Value) -> Result<Value> {
    let prompt = input["prompt"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .context("请输入生成描述")?;
    let mut parts = vec![json!({"text":prompt})];
    if let Some(images) = input.get("image_urls") {
        let images = images.as_array().context("参考图片格式无效")?;
        for image in images {
            let url = image.as_str().context("参考图片格式无效")?;
            let (mime, _) = image_bytes(url)?;
            parts.push(
                json!({"inlineData":{"mimeType":mime,"data":url.split_once(',').unwrap().1}}),
            );
        }
    }
    let mut config = input.get("generationConfig").cloned().unwrap_or(json!({}));
    ensure!(config.is_object(), "生成配置须为对象");
    config["responseModalities"] = json!(["TEXT", "IMAGE"]);
    let body = json!({"contents":[{"role":"user","parts":parts}],"generationConfig":config});
    ensure!(
        body.to_string().len() <= 20_000_000,
        "参考图片总量过大，请减少图片"
    );
    Ok(body)
}
fn parse(value: &Value) -> Result<super::ProviderUpdate> {
    let mut outputs = vec![];
    for part in value["candidates"][0]["content"]["parts"]
        .as_array()
        .into_iter()
        .flatten()
    {
        if part["thought"] == true {
            continue;
        }
        if let (Some(mime), Some(data)) = (
            part["inlineData"]["mimeType"].as_str(),
            part["inlineData"]["data"].as_str(),
        ) {
            let url = format!("data:{mime};base64,{data}");
            image_bytes(&url)?;
            outputs.push(ModelOutput {
                kind: "image".into(),
                url,
            });
        }
    }
    ensure!(
        !outputs.is_empty(),
        "Gemini 未返回图片，请检查模型能力或内容限制"
    );
    Ok(serde_json::from_value(
        json!({"status":"COMPLETED","result":{"outputs":outputs}}),
    )?)
}
impl ProviderAdapter for Gemini {
    fn id(&self) -> &'static str {
        "gemini-native"
    }
    fn validate_endpoint(&self, value: &str) -> Result<()> {
        endpoint(value)
    }
    fn submit<'a>(
        &'a self,
        key: &'a str,
        base: &'a str,
        input: &'a Value,
        _: &'a Value,
        storage_lease: super::StorageLease,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            let _storage_lease = storage_lease;
            endpoint(base)?;
            ensure!(!key.is_empty(), "请配置 Google API Key");
            let model = input["model"].as_str().context("缺少 Gemini 模型 ID")?;
            ensure!(
                !model.is_empty()
                    && model
                        .bytes()
                        .all(|c| c.is_ascii_alphanumeric() || b"-._".contains(&c)),
                "模型 ID 无效"
            );
            let root = base.trim_end_matches('/');
            let root = if root.ends_with("/v1beta") || root.ends_with("/v1") {
                root.into()
            } else {
                format!("{root}/v1beta")
            };
            let mut response = crate::jobs::client()?
                .post(format!("{root}/models/{model}:generateContent"))
                .header("x-goog-api-key", key)
                .json(&payload(input)?)
                .send()
                .await
                .map_err(|_| anyhow::anyhow!("Google 请求中断，结果未知，请核查后重试"))?;
            if !response.status().is_success() {
                return Err(crate::app_error::http_error(response, "submit")
                    .await
                    .into());
            }
            let mut bytes = Vec::new();
            while let Some(chunk) = response.chunk().await? {
                ensure!(
                    bytes.len() + chunk.len() <= 60_000_000,
                    "Google 图片响应过大"
                );
                bytes.extend_from_slice(&chunk);
            }
            parse(&serde_json::from_slice(&bytes)?)
        })
    }
    fn refresh<'a>(&'a self, _: &'a str, job: &'a Value) -> ProviderFuture<'a> {
        Box::pin(async move {
            ensure!(
                job["status"] == "COMPLETED",
                "Google 同步请求无法轮询，请到服务端核查结果"
            );
            Ok(serde_json::from_value(json!({"status":"COMPLETED"}))?)
        })
    }
    fn cancel<'a>(&'a self, _: &'a str, _: &'a Value) -> ProviderFuture<'a> {
        Box::pin(async { anyhow::bail!("Google 同步图像请求不支持远程取消") })
    }
    fn outputs(&self, result: &Value) -> Vec<ModelOutput> {
        serde_json::from_value(result["outputs"].clone()).unwrap_or_default()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn native_images_roundtrip_and_reject_invalid_payloads() {
        let image = "data:image/png;base64,YQ==";
        let body = payload(&json!({"prompt":"edit", "image_urls":[image]})).unwrap();
        assert_eq!(
            body["contents"][0]["parts"][1]["inlineData"]["data"],
            "YQ=="
        );
        assert_eq!(
            body["generationConfig"]["responseModalities"],
            json!(["TEXT", "IMAGE"])
        );
        let update = parse(&json!({"candidates":[{"content":{"parts":[{"thought":true,"inlineData":{"mimeType":"image/png","data":"Yg=="}},{"inlineData":{"mimeType":"image/png","data":"YQ=="}}]}}]})).unwrap();
        assert_eq!(Gemini.outputs(&update.result.unwrap())[0].url, image);
        let many = payload(&json!({"prompt":"reference", "image_urls":vec![image;16]})).unwrap();
        assert_eq!(many["contents"][0]["parts"].as_array().unwrap().len(), 17);
        assert!(parse(&json!({"candidates":[]})).is_err());
        assert!(
            payload(&json!({"prompt":"edit","image_urls":["https://example.com/a.png"]})).is_err()
        );
        assert!(endpoint("https://evil.example").is_err());
        assert!(image_bytes("data:text/html;base64,YQ==").is_err());
    }
}
