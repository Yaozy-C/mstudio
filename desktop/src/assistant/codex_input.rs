//! Keep image bytes out of the serialized text history sent to app-server.
use anyhow::{Result, bail};
use base64::{Engine, engine::general_purpose::STANDARD};
use rig_core::message::{
    AssistantContent, DocumentSourceKind, Image, Message, MimeType, ToolResultContent, UserContent,
};
use serde_json::{Value, json};

pub const TEXT_LIMIT: usize = 1_048_576;

fn attach(image: &Image, items: &mut Vec<Value>) -> Result<String> {
    let mut item = json!({"type":"image"});
    match &image.data {
        DocumentSourceKind::Url(url) => item["url"] = json!(url),
        DocumentSourceKind::FileId(id) => item["fileId"] = json!(id),
        DocumentSourceKind::Base64(data) => {
            let mime = image
                .media_type
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Codex 图片缺少媒体类型"))?;
            item["url"] = json!(format!("data:{};base64,{data}", mime.to_mime_type()));
        }
        DocumentSourceKind::Raw(bytes) => {
            let mime = image
                .media_type
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("Codex 图片缺少媒体类型"))?;
            item["url"] = json!(format!(
                "data:{};base64,{}",
                mime.to_mime_type(),
                STANDARD.encode(bytes)
            ));
        }
        _ => bail!("Codex 不支持此图片来源"),
    }
    let label = format!("Mstudio image {} (attached below)", items.len() / 2 + 1);
    items.push(json!({"type":"text","text":label}));
    items.push(item);
    Ok(label)
}

pub fn turn_input(messages: &[Message]) -> Result<Value> {
    let mut history = messages.to_vec();
    let mut images = Vec::new();
    for message in &mut history {
        match message {
            Message::User { content } => {
                for part in content {
                    match part {
                        UserContent::Image(image) => {
                            *part = UserContent::text(attach(image, &mut images)?)
                        }
                        UserContent::ToolResult(result) => {
                            for part in &mut result.content {
                                if let ToolResultContent::Image(image) = part {
                                    *part = ToolResultContent::text(attach(image, &mut images)?);
                                }
                            }
                        }
                        UserContent::Audio(_)
                        | UserContent::Video(_)
                        | UserContent::Document(_) => {
                            bail!("Codex 对话仅支持文字和图片，请改用支持该资料类型的模型")
                        }
                        _ => {}
                    }
                }
            }
            Message::Assistant { content, .. } => {
                for part in content {
                    if let AssistantContent::Image(image) = part {
                        *part = AssistantContent::text(attach(image, &mut images)?);
                    }
                }
            }
            _ => {}
        }
    }
    let text = serde_json::to_string(&history)?;
    let total = text.chars().count()
        + images
            .iter()
            .filter_map(|i| i["text"].as_str())
            .map(|s| s.chars().count())
            .sum::<usize>();
    anyhow::ensure!(
        total <= TEXT_LIMIT,
        "Codex 文字上下文超过输入长度限制，请压缩对话后继续"
    );
    let mut input = vec![json!({"type":"text","text":text})];
    input.extend(images);
    Ok(json!(input))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig_core::message::ImageMediaType;
    #[test]
    fn large_images_are_native_inputs_and_tool_results_keep_their_identity() {
        let data = STANDARD.encode(vec![0_u8; 1_000_000]);
        let image = Image {
            data: DocumentSourceKind::Base64(data.clone()),
            media_type: Some(ImageMediaType::PNG),
            ..Default::default()
        };
        let mut tool = Message::tool_result("call-1", "read_image", "image follows");
        if let Message::User { content } = &mut tool
            && let UserContent::ToolResult(result) = &mut content[0]
        {
            result.content.push(ToolResultContent::Image(image));
        }
        let messages = vec![
            Message::User {
                content: vec![
                    UserContent::text("Inspect references"),
                    UserContent::image_base64(&data, Some(ImageMediaType::PNG), None),
                ],
            },
            tool,
        ];
        let input = turn_input(&messages).unwrap();
        let text = input[0]["text"].as_str().unwrap();
        assert!(text.len() < 2000);
        assert!(!text.contains(&data));
        assert!(text.contains("call-1") && text.contains("read_image"));
        assert!(text.contains("Mstudio image 1") && text.contains("Mstudio image 2"));
        assert_eq!(input[2]["url"], format!("data:image/png;base64,{data}"));
        assert_eq!(input[4]["url"], input[2]["url"]);
        assert!(serde_json::to_string(&messages).unwrap().contains(&data));
    }
    #[test]
    fn oversized_text_is_not_silently_truncated() {
        assert!(
            turn_input(&[Message::user("x".repeat(TEXT_LIMIT))])
                .unwrap_err()
                .to_string()
                .contains("压缩对话")
        );
    }
}
