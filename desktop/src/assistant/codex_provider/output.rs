//! Native dynamic-tool responses keep typed images and tool failures intact.
use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use rig_core::{
    message::{AdditionalParams, DocumentSourceKind, MimeType, ToolResult, ToolResultContent},
    streaming::{RawStreamingChoice, StreamPartId},
};
use serde_json::{Value, json};

pub(super) fn text_start(item: &Value) -> Option<RawStreamingChoice> {
    Some(RawStreamingChoice::TextStart {
        id: StreamPartId::wire(item["id"].as_str()?),
        additional_params: AdditionalParams::from_entries(
            item["phase"].as_str().map(|p| ("phase", json!(p))),
        ),
    })
}
pub(super) fn tool_response(result: &ToolResult) -> Result<Value> {
    let mut items = Vec::new();
    let mut success = true;
    for part in &result.content {
        match part {
            ToolResultContent::Text(text) => {
                if let Ok(value) = serde_json::from_str::<Value>(&text.text) {
                    success &= value.get("error").is_none_or(Value::is_null)
                        && value["ok"] != false
                        && value["isError"] != true;
                }
                items.push(json!({"type":"inputText","text":text.text}));
            }
            ToolResultContent::Image(image) => {
                let url = match &image.data {
                    DocumentSourceKind::Url(url) if url.starts_with("file:") => {
                        let path = reqwest::Url::parse(url)?
                            .to_file_path()
                            .map_err(|_| anyhow::anyhow!("图片路径无效"))?;
                        let ext = path
                            .extension()
                            .and_then(|v| v.to_str())
                            .unwrap_or("")
                            .to_lowercase();
                        let (mime, bytes) = crate::assistant::image_input::read(&path, &ext)?;
                        format!("data:{mime};base64,{}", STANDARD.encode(bytes))
                    }
                    DocumentSourceKind::Url(url) => url.clone(),
                    DocumentSourceKind::Base64(data) => format!(
                        "data:{};base64,{data}",
                        image
                            .media_type
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("图片缺少媒体类型"))?
                            .to_mime_type()
                    ),
                    DocumentSourceKind::Raw(data) => format!(
                        "data:{};base64,{}",
                        image
                            .media_type
                            .as_ref()
                            .ok_or_else(|| anyhow::anyhow!("图片缺少媒体类型"))?
                            .to_mime_type(),
                        STANDARD.encode(data)
                    ),
                    _ => bail!("Codex 工具结果包含不支持的图片来源"),
                };
                items.push(json!({"type":"inputImage","imageUrl":url}));
            }
            _ => bail!("Codex 工具结果仅支持文字和图片"),
        }
    }
    Ok(json!({"success":success,"contentItems":items}))
}
