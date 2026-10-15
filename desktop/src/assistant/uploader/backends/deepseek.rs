use super::{Backend, ImageBudget, Result, Source, error, response_image};
use super::{UploadFuture, UploadRequest};
use serde_json::{Value, json};

pub struct DeepSeek;

impl Backend for DeepSeek {
    fn upload<'a>(&'a self, request: UploadRequest<'a>, source: &'a Source) -> UploadFuture<'a> {
        Box::pin(super::super::upload::openai_file(self, request, source))
    }
    fn headers(&self, path: &str, headers: &mut rig_core::http_client::HeaderMap) {
        if path.ends_with("/messages") {
            let enabled = headers.get_all("anthropic-beta").iter().any(|value| {
                value.to_str().is_ok_and(|value| {
                    value
                        .split(',')
                        .any(|beta| beta.trim() == "files-api-2025-04-14")
                })
            });
            if !enabled {
                headers.append("anthropic-beta", "files-api-2025-04-14".parse().unwrap());
            }
        }
    }
    fn name(&self) -> &'static str {
        "deepseek"
    }
    fn root(&self) -> &'static str {
        "https://api.deepseek.com"
    }
    fn purpose(&self, _: &Source) -> &'static str {
        "user_data"
    }
    fn id_prefix(&self) -> &'static str {
        "file-api-"
    }
    fn max_file_bytes(&self) -> usize {
        64 * 1024 * 1024
    }
    fn image_budget(&self) -> ImageBudget {
        ImageBudget {
            bytes: 200 * 1024 * 1024,
            count: 600,
        }
    }
    fn handles(&self, path: &str) -> bool {
        ["/responses", "/chat/completions", "/messages"]
            .iter()
            .any(|suffix| path.ends_with(suffix))
    }
    fn project(&self, part: &mut Value, id: &str) -> Result<()> {
        match part["type"].as_str() {
            Some("input_image") => return response_image(part, id),
            Some("image_url") => *part = json!({"type":"file","file_id":id}),
            Some("image") => *part = json!({"type":"image","source":{"type":"file","file_id":id}}),
            _ => return Err(error("DeepSeek 不支持此图片引用协议")),
        }
        Ok(())
    }
}
