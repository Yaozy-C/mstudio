use super::{Error, Result, Source, error};
use base64::{Engine, engine::general_purpose::STANDARD};
use serde_json::Value;

fn image(part: &Value) -> Result<Option<Source>> {
    let source = match part["type"].as_str() {
        Some("input_image") => part["image_url"].as_str(),
        Some("image_url") => part["image_url"]["url"].as_str(),
        Some("image") if part["source"]["type"] == "base64" => {
            let mime = part["source"]["media_type"]
                .as_str()
                .ok_or_else(|| error("图片缺少媒体类型"))?;
            let data = part["source"]["data"]
                .as_str()
                .ok_or_else(|| error("图片缺少内容"))?;
            return decoded(mime, data).map(Some);
        }
        _ => None,
    };
    let Some(source) = source.filter(|source| source.starts_with("data:")) else {
        return Ok(None);
    };
    let (header, data) = source
        .split_once(',')
        .ok_or_else(|| error("图片 data URL 无效"))?;
    let mime = header
        .strip_prefix("data:")
        .and_then(|s| s.strip_suffix(";base64"))
        .ok_or_else(|| error("图片必须使用 Base64 data URL"))?;
    decoded(mime, data).map(Some)
}

fn decoded(mime: &str, data: &str) -> Result<Source> {
    Source::new(mime, STANDARD.decode(data).map_err(Error::instance)?)
}

pub(super) fn collect(body: &Value) -> Result<Vec<(String, Source)>> {
    fn visit(value: &Value, path: &str, found: &mut Vec<(String, Source)>) -> Result<()> {
        if let Some(image) = image(value)? {
            found.push((path.into(), image));
            return Ok(());
        }
        match value {
            Value::Array(items) => {
                for (index, item) in items.iter().enumerate() {
                    visit(item, &format!("{path}/{index}"), found)?;
                }
            }
            Value::Object(fields) => {
                for (key, item) in fields {
                    if ["content", "output", "messages", "input"].contains(&key.as_str()) {
                        visit(item, &format!("{path}/{key}"), found)?;
                    }
                }
            }
            _ => {}
        }
        Ok(())
    }
    let mut found = Vec::new();
    visit(body, "", &mut found)?;
    Ok(found)
}
