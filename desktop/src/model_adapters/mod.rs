//! Provider transport is separate from model capabilities and project job persistence.
mod codex;
pub(crate) mod codex_discovery;
mod codex_request;
pub mod codex_rpc;
pub mod codex_setup;
mod fal;
pub mod prompt_rules;
pub(crate) use fal::recover_cancelled;
pub mod gemini;
pub mod http_json;
pub mod image_data;
use anyhow::{Result, bail};
#[cfg(test)]
pub use fal::queue_url;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{future::Future, pin::Pin};

// Native background providers retain this lease until their last workspace write.
pub type StorageLease = Option<tokio::sync::OwnedRwLockReadGuard<()>>;
pub type ProviderFuture<'a> = Pin<Box<dyn Future<Output = Result<ProviderUpdate>> + Send + 'a>>;
#[derive(Debug, Serialize, Deserialize, PartialEq)]
pub struct ModelOutput {
    pub kind: String,
    pub url: String,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderUpdate {
    pub status: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub progress: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub request_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub response_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancel_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outputs: Option<Vec<ModelOutput>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}
impl ProviderUpdate {
    pub fn apply(self, job: &mut Value) -> Result<()> {
        if let Some(outputs) = &self.outputs {
            job["outputCount"] = serde_json::json!(outputs.len());
        }
        let update = serde_json::to_value(self)?;
        job.as_object_mut()
            .ok_or_else(|| anyhow::anyhow!("任务格式无效"))?
            .extend(update.as_object().unwrap().clone());
        Ok(())
    }
}
/// Every provider returns the same job update fields and normalized output records.
/// Only this layer sees provider authentication, queue URLs and wire responses.
pub trait ProviderAdapter: Sync {
    fn id(&self) -> &'static str;
    fn validate_endpoint(&self, endpoint: &str) -> Result<()>;
    fn submit<'a>(
        &'a self,
        key: &'a str,
        endpoint: &'a str,
        input: &'a Value,
        config: &'a Value,
        storage_lease: StorageLease,
    ) -> ProviderFuture<'a>;
    fn refresh<'a>(&'a self, key: &'a str, job: &'a Value) -> ProviderFuture<'a>;
    fn cancel<'a>(&'a self, key: &'a str, job: &'a Value) -> ProviderFuture<'a>;
    fn outputs(&self, result: &Value) -> Vec<ModelOutput>;
}
static HTTP_JSON: http_json::HttpJson = http_json::HttpJson;
static GEMINI: gemini::Gemini = gemini::Gemini;
static FAL: fal::Fal = fal::Fal;
pub fn provider(id: &str) -> Result<&'static dyn ProviderAdapter> {
    match id {
        "codex-image" => Ok(&codex::CODEX),
        "fal" => Ok(&FAL),
        "gemini-native" => Ok(&GEMINI),
        "http-json" => Ok(&HTTP_JSON),
        _ => bail!("服务供应商适配器尚未安装：{id}"),
    }
}
pub fn for_job(job: &Value) -> Result<&'static dyn ProviderAdapter> {
    provider(
        job["providerId"]
            .as_str()
            .ok_or_else(|| anyhow::anyhow!("任务缺少服务供应商"))?,
    )
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn refresh_update_preserves_durable_provider_handles() {
        let mut job = json!({"requestId":"existing","statusUrl":"https://queue.fal.run/task/status","providerId":"fal"});
        let update: ProviderUpdate =
            serde_json::from_value(json!({"status":"IN_PROGRESS"})).unwrap();
        update.apply(&mut job).unwrap();
        assert_eq!(job["requestId"], "existing");
        assert_eq!(job["providerId"], "fal");
        assert_eq!(job["status"], "IN_PROGRESS");
    }
    #[test]
    fn missing_or_unknown_providers_are_rejected() {
        assert!(for_job(&json!({})).is_err());
        assert!(for_job(&json!({"providerId":"not-installed"})).is_err());
    }
    #[test]
    fn provider_outputs_keep_all_images_in_a_common_shape() {
        let output = provider("fal").unwrap().outputs(&json!({"images":[{"url":"https://example.com/1.png"},{"url":"https://example.com/2.png"}]}));
        assert_eq!(output.len(), 2);
        assert_eq!(
            output[1],
            ModelOutput {
                kind: "image".into(),
                url: "https://example.com/2.png".into()
            }
        );
        assert!(
            provider("fal")
                .unwrap()
                .outputs(&json!({"error":"bad"}))
                .is_empty()
        );
    }
}
