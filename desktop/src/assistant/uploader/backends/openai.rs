use super::{Backend, ImageBudget, Result, Source, response_image};
use super::{UploadFuture, UploadRequest};
use serde_json::Value;

pub struct OpenAi;

impl Backend for OpenAi {
    fn upload<'a>(&'a self, request: UploadRequest<'a>, source: &'a Source) -> UploadFuture<'a> {
        Box::pin(super::super::upload::openai_file(self, request, source))
    }
    fn name(&self) -> &'static str {
        "openai"
    }
    fn root(&self) -> &'static str {
        "https://api.openai.com/v1"
    }
    fn purpose(&self, source: &Source) -> &'static str {
        if source.mime.starts_with("image/") {
            "vision"
        } else {
            "user_data"
        }
    }
    fn id_prefix(&self) -> &'static str {
        "file-"
    }
    fn max_file_bytes(&self) -> usize {
        512 * 1024 * 1024
    }
    fn image_budget(&self) -> ImageBudget {
        ImageBudget {
            bytes: 512 * 1024 * 1024,
            count: 1500,
        }
    }
    fn handles(&self, path: &str) -> bool {
        path.ends_with("/responses")
    }
    fn project(&self, part: &mut Value, id: &str) -> Result<()> {
        response_image(part, id)
    }
}
