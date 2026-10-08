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
        DocumentSourceKind::Url(url) => {
            if url.starts_with("file:") {
                let path = reqwest::Url::parse(url)?
                    .to_file_path()
                    .map_err(|_| anyhow::anyhow!("Codex 本地图片路径无效"))?;
                anyhow::ensure!(path.is_file(), "Codex 本地图片已丢失，请重新选择素材");
                item = json!({"type":"localImage","path":path});
            } else {
                item["url"] = json!(url);
            }
        }
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
    fn project_images_use_local_files_and_remote_urls_stay_urls() {
        let (root, store, mut doc) = super::super::attachment_tests::fixture();
        let path = root.join("assets/产品 reference #1.png");
        let file = std::fs::File::create(&path).unwrap();
        file.set_len(20 * 1024 * 1024).unwrap();
        let mut asset: mstudio::model::Asset =
            serde_json::from_value(doc["assets"][0].clone()).unwrap();
        asset.path = path.to_string_lossy().into();
        asset.id = "large-local".into();
        crate::project_storage::save_asset(&store, "p", &asset).unwrap();
        doc["assets"][0] = json!(asset);
        let mut profile = super::super::config::Profile {
            adapter: "codex".into(),
            ..Default::default()
        };
        profile.inputs.image = true;
        let payload = super::super::attachments::payload(
            &store,
            &doc,
            "检查原图",
            &[super::super::attachments::Reference {
                kind: "asset".into(),
                id: asset.id,
            }],
            &profile,
        )
        .unwrap();
        assert!(payload.to_string().len() < 4096);
        assert!(!payload.to_string().contains("base64"));
        assert!(
            !super::super::context::text_only(&payload)
                .to_string()
                .contains("file:")
        );
        let message = super::super::agent::convert(&json!({"role":"user","content":payload}));
        let mut tool = Message::tool_result("read-1", "read_image", "reference");
        if let Message::User { content } = &mut tool
            && let UserContent::ToolResult(result) = &mut content[0]
        {
            result.content.push(ToolResultContent::Image(Image {
                data: DocumentSourceKind::Url(reqwest::Url::from_file_path(&path).unwrap().into()),
                ..Default::default()
            }));
        }
        let input = turn_input(&[
            message,
            tool,
            Message::User {
                content: vec![UserContent::image_url(
                    "https://example.com/reference.png",
                    None,
                    None,
                )],
            },
        ])
        .unwrap();
        assert_eq!(
            input[2],
            json!({"type":"localImage", "path":path.canonicalize().unwrap()})
        );
        assert_eq!(input[4], json!({"type":"localImage", "path":path}));
        assert_eq!(
            input[6],
            json!({"type":"image", "url":"https://example.com/reference.png"})
        );
        assert_eq!(std::fs::metadata(&path).unwrap().len(), 20 * 1024 * 1024);
        std::fs::remove_file(&path).unwrap();
        let mut images = vec![];
        assert!(
            attach(
                &Image {
                    data: DocumentSourceKind::Url(
                        reqwest::Url::from_file_path(&path).unwrap().into()
                    ),
                    ..Default::default()
                },
                &mut images
            )
            .is_err()
        );
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
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
