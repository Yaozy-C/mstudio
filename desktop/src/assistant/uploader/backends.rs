pub use super::upload::UploadRequest;
use super::{Result, Source, Uploaded, error};
use std::{future::Future, pin::Pin};
pub type UploadFuture<'a> = Pin<Box<dyn Future<Output = Result<Uploaded>> + Send + 'a>>;
use crate::assistant::config::Profile;
use serde_json::Value;
use std::sync::Arc;

pub mod deepseek;
pub mod openai;
#[cfg(test)]
mod tests;

pub struct ImageBudget {
    pub bytes: usize,
    pub count: usize,
}

/// Remote upload capability, independent of the model's wire protocol.
pub trait Backend: Send + Sync {
    fn upload<'a>(&'a self, request: UploadRequest<'a>, source: &'a Source) -> UploadFuture<'a>;
    fn headers(&self, _path: &str, _headers: &mut rig_core::http_client::HeaderMap) {}
    fn name(&self) -> &'static str;
    fn root(&self) -> &'static str;
    fn purpose(&self, source: &Source) -> &'static str;
    fn id_prefix(&self) -> &'static str;
    fn max_file_bytes(&self) -> usize;
    fn image_budget(&self) -> ImageBudget;
    fn handles(&self, path: &str) -> bool;
    fn project(&self, part: &mut Value, id: &str) -> Result<()>;

    fn validate(&self, source: &Source) -> Result<()> {
        if source.bytes.len() > self.max_file_bytes() {
            return Err(error("文件超过上传平台的单文件大小限制"));
        }
        if source.mime.starts_with("image/")
            && !["image/png", "image/jpeg", "image/gif", "image/webp"]
                .contains(&source.mime.as_str())
        {
            return Err(error("图片上传仅支持 PNG、JPEG、GIF、WebP"));
        }
        Ok(())
    }
}

pub fn select(profile: &Profile) -> Option<Arc<dyn Backend>> {
    let url = reqwest::Url::parse(&profile.endpoint).ok()?;
    if url.scheme() != "https" || url.port().is_some() {
        return None;
    }
    match (url.host_str()?, profile.adapter.as_str()) {
        ("api.deepseek.com", "openai-compatible" | "openai-responses" | "anthropic-native") => {
            Some(Arc::new(deepseek::DeepSeek))
        }
        ("api.openai.com", "openai-responses") => Some(Arc::new(openai::OpenAi)),
        // Codex app-server accepts existing image fileId inputs, but exposes no
        // general upload operation returning one. Its ChatGPT auth stays local.
        _ => None,
    }
}

fn response_image(part: &mut Value, id: &str) -> Result<()> {
    if part["type"] != "input_image" {
        return Err(error("上传平台不支持此图片引用协议"));
    }
    let fields = part.as_object_mut().ok_or_else(|| error("图片内容无效"))?;
    fields.remove("image_url");
    fields.insert("file_id".into(), Value::String(id.into()));
    Ok(())
}
