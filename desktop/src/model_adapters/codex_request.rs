use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

fn config() -> Result<Value> {
    serde_json::from_str(include_str!(
        "../../../frontend/src/models/config/native-requests.json"
    ))
    .context("原生请求配置无效")
}
fn default<'a>(transport: &'a Value, key: &str) -> Result<&'a str> {
    transport["defaults"][key]
        .as_str()
        .with_context(|| format!("原生请求配置缺少默认值 {key}"))
}
fn valid_size(value: &str) -> bool {
    let Some((width, height)) = value.split_once('x') else {
        return false;
    };
    let (Ok(width), Ok(height)) = (width.parse::<u64>(), height.parse::<u64>()) else {
        return false;
    };
    width > 0
        && height > 0
        && width % 16 == 0
        && height % 16 == 0
        && width <= 3840
        && height <= 3840
        && width * height >= 655_360
        && width * height <= 8_294_400
        && width * 3 >= height
        && height * 3 >= width
}
fn size<'a>(input: &'a Value, transport: &'a Value) -> Result<&'a str> {
    let value = match input.get("size") {
        Some(value) => value.as_str().context("size 必须是字符串")?,
        None => default(transport, "size")?,
    };
    ensure!(value != "auto", "请指定明确的 size，不要使用 auto");
    ensure!(
        valid_size(value),
        "size 须为 WIDTHxHEIGHT：宽高为 16 的倍数，比例 1:3–3:1，总像素 655360–8294400"
    );
    Ok(value)
}
fn quality<'a>(input: &'a Value, transport: &'a Value) -> Result<&'a str> {
    let value = match input.get("quality") {
        Some(value) => value.as_str().context("quality 必须是字符串")?,
        None => default(transport, "quality")?,
    };
    ensure!(
        value != "auto",
        "请指定明确的 quality，不要使用 auto"
    );
    let values = transport["qualityValues"]
        .as_array()
        .context("原生请求配置缺少 qualityValues")?;
    ensure!(
        values.iter().any(|allowed| allowed.as_str() == Some(value)),
        "不支持的 quality：{value}"
    );
    Ok(value)
}

/// Requests must carry an explicit model ID.
pub fn model(input: &Value) -> Result<&str> {
    let id = input["model"]
        .as_str()
        .context("请填写有效的 Codex 图像模型 ID")?;
    ensure!(
        id.starts_with("gpt-image-")
            && id.len() > "gpt-image-".len()
            && id.len() <= 100
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || b"-._".contains(&c)),
        "请填写有效的 Codex 图像模型 ID"
    );
    Ok(id)
}

pub fn validate(input: &Value) -> Result<()> {
    let object = input.as_object().context("请求必须是 JSON 对象")?;
    model(input)?;
    let config = config()?;
    let transport = &config["codex-image"];
    let allowed = transport["allowedFields"]
        .as_array()
        .context("原生请求配置无效")?;
    for key in object.keys() {
        ensure!(
            allowed.iter().any(|field| field == key),
            "原生连接不支持参数：{key}"
        );
    }
    let prompt = input["prompt"].as_str().context("请输入生成描述")?;
    ensure!(
        !prompt.trim().is_empty() && prompt.len() <= 32000,
        "生成描述须为 1–32000 字节"
    );
    for (key, expected) in transport["fixedValues"]
        .as_object()
        .context("原生请求配置无效")?
    {
        ensure!(
            input.get(key).is_none_or(|v| v == expected),
            "原生连接不支持指定 {key}；当前只支持 {expected}"
        );
    }
    size(input, transport)?;
    quality(input, transport)?;
    if let Some(n) = input.get("n") {
        ensure!(
            n.as_u64().is_some_and(|n| (1..=10).contains(&n)),
            "图片张数须为 1–10 的整数"
        );
    }
    if let Some(images) = input.get("image") {
        let images = images.as_array().context("image 必须为内嵌图片数组")?;
        ensure!(!images.is_empty(), "编辑至少需要 1 张参考图片");
        let mut total = 0;
        for image in images {
            let url = image.as_str().context("参考图片格式无效")?;
            total += url.len();
            ensure!(total <= 25_000_000, "参考图片总量超过限制");
            super::image_data::image_bytes(url)?;
        }
    }
    Ok(())
}
/// Construct the direct wire payload without changing a single prompt character.
pub fn payload(input: &Value) -> Result<(&'static str, Value)> {
    validate(input)?;
    let config = config()?;
    let transport = &config["codex-image"];
    let size = size(input, transport)?;
    let quality = quality(input, transport)?;
    let mut body = json!({"model":model(input)?, "prompt":input["prompt"],
        "background":"opaque", "quality":quality, "size":size});
    let endpoint = if let Some(images) = input["image"].as_array() {
        body["images"] = images
            .iter()
            .map(|image| json!({"image_url":image}))
            .collect();
        "images/edits"
    } else {
        "images/generations"
    };
    Ok((endpoint, body))
}
pub fn image_result(response: &Value) -> Result<Vec<Value>> {
    let data = response["data"]
        .as_array()
        .context("Codex 未返回图片数据")?;
    ensure!(!data.is_empty(), "Codex 未返回图片数据");
    data.iter()
        .map(|item| {
            let encoded = item["b64_json"]
                .as_str()
                .filter(|s| !s.is_empty())
                .context("Codex 未返回图片数据")?;
            let (_, bytes) =
                super::image_data::image_bytes(&format!("data:image/png;base64,{encoded}"))?;
            ensure!(
                bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
                "Codex 返回了非 PNG 图片"
            );
            Ok(json!({"b64_json":encoded}))
        })
        .collect()
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn selected_model_is_sent_for_generation_and_edit_without_substitution() {
        for id in [
            "gpt-image-2",
            "gpt-image-2.5-sunburst",
            "gpt-image-2.5-flare",
        ] {
            for edit in [false, true] {
                let mut input = json!({"prompt":"x","model":id});
                if edit {
                    input["image"] = json!(["data:image/png;base64,YQ=="]);
                }
                let (path, body) = payload(&input).unwrap();
                assert_eq!(body["model"], id);
                assert_eq!(
                    path,
                    if edit {
                        "images/edits"
                    } else {
                        "images/generations"
                    }
                );
            }
        }
        for input in [
            json!({"prompt":"x"}),
            json!({"prompt":"x","model":"codex-image"}),
        ] {
            assert!(payload(&input).unwrap_err().to_string().contains("模型 ID"));
        }
    }
    #[test]
    fn direct_payload_preserves_prompt_and_every_reference() {
        let prompt = "  中文\n\"Do not rewrite\" \t";
        let input = json!({"prompt":prompt,"model":"gpt-image-2.5-sunburst","image":vec!["data:image/png;base64,YQ==";12],"n":3});
        let (path, body) = payload(&input).unwrap();
        assert_eq!(path, "images/edits");
        assert_eq!(body["prompt"], prompt);
        assert_eq!(body["images"].as_array().unwrap().len(), 12);
        assert_eq!(body["images"][11]["image_url"], input["image"][11]);
        assert!(body.get("n").is_none()); // Each job sends one request per output.
        assert_eq!(body["size"], "2048x1152");
        assert_eq!(body["quality"], "high");
        assert_eq!(
            payload(&json!({"prompt":"x","model":"gpt-image-2"}))
                .unwrap()
                .0,
            "images/generations"
        );
    }
    #[test]
    fn explicit_size_and_quality_are_sent_and_auto_is_rejected() {
        let (_, body) = payload(&json!({
            "prompt":"x",
            "model":"gpt-image-2.5-sunburst",
            "size":"1536x864",
            "quality":"xhigh"
        }))
        .unwrap();
        assert_eq!(body["size"], "1536x864");
        assert_eq!(body["quality"], "xhigh");
        for input in [
            json!({"prompt":"x","size":"auto"}),
            json!({"prompt":"x","quality":"auto"}),
            json!({"prompt":"x","size":"1920x1080"}),
            json!({"prompt":"x","size":"1000x1000"}),
            json!({"prompt":"x","size":"4096x4096"}),
            json!({"prompt":"x","size":"4096x1000"}),
            json!({"prompt":"x","size":1024}),
            json!({"prompt":"x","quality":"ultra"}),
            json!({"prompt":"x","quality":42}),
        ] {
            let mut explicit = input.clone();
            explicit["model"] = json!("gpt-image-2.5-sunburst");
            assert!(validate(&explicit).is_err(), "{explicit}");
        }
    }
    #[test]
    fn rejects_unsupported_controls_and_invalid_results() {
        for input in [
            json!({"prompt":"x","n":0}),
            json!({"prompt":"x","n":11}),
            json!({"prompt":"x","n":1.5}),
            json!({"prompt":"x","model":""}),
            json!({"prompt":"x","model":null}),
            json!({"prompt":"x","model":42}),
            json!({"prompt":"x","model":"gpt-image-"}),
            json!({"prompt":"x","model":"gpt-image-2\n"}),
            json!({"prompt":"x","model":"gpt-image-2/other"}),
            json!({"prompt":"x","model":format!("gpt-image-{}", "x".repeat(100))}),
            json!({"prompt":"x","image":[]}),
            json!({"prompt":"x","output_format":"jpeg"}),
        ] {
            let mut explicit = input.clone();
            if explicit.get("model").is_none() {
                explicit["model"] = json!("gpt-image-2");
            }
            assert!(validate(&explicit).is_err());
        }
        assert!(image_result(&json!({"data":[]})).is_err());
        assert!(image_result(&json!({"data":[{"b64_json":"YQ=="}]})).is_err());
        assert_eq!(
            image_result(&json!({"data":[{"b64_json":"iVBORw0KGgo="}]}))
                .unwrap()
                .len(),
            1
        );
    }
}
