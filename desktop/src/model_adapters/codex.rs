use super::{
    ModelOutput, ProviderAdapter, ProviderFuture, ProviderUpdate, codex_auth, codex_http,
    codex_request,
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{LazyLock, Mutex},
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio_util::sync::CancellationToken;
pub struct Codex;
pub static CODEX: Codex = Codex;
static SLOTS: tokio::sync::Semaphore = tokio::sync::Semaphore::const_new(2);
static ACTIVE: LazyLock<Mutex<HashMap<String, CancellationToken>>> =
    LazyLock::new(Default::default);
fn update(status: &str, result: Option<Value>, error: Option<String>) -> ProviderUpdate {
    ProviderUpdate {
        status: status.into(),
        progress: None,
        request_id: None,
        status_url: None,
        response_url: None,
        cancel_url: None,
        outputs: result.as_ref().map(|r| CODEX.outputs(r)),
        result,
        error,
    }
}
fn directory(config: &Value, id: &str) -> Result<PathBuf> {
    ensure!(
        !id.is_empty()
            && id.len() <= 80
            && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "Codex 任务编号无效"
    );
    Ok(PathBuf::from(
        config["codexRoot"]
            .as_str()
            .context("缺少 Codex 任务目录")?,
    )
    .join(id))
}
fn persist(root: &std::path::Path, value: &ProviderUpdate) -> Result<()> {
    let pending = root.join("state.tmp");
    std::fs::write(&pending, serde_json::to_vec(value)?)?;
    std::fs::rename(pending, root.join("state.json"))?;
    Ok(())
}
fn phase(root: &std::path::Path, stage: &str, message: &str) -> Result<()> {
    let previous = std::fs::read(root.join("state.json"))
        .ok()
        .and_then(|s| serde_json::from_slice::<ProviderUpdate>(&s).ok());
    let mut state = update("IN_PROGRESS", previous.and_then(|s| s.result), None);
    state.progress = Some(
        json!({"stage":stage,"message":message,"updatedAt":SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs()}),
    );
    persist(root, &state)
}
fn read(job: &Value) -> Result<ProviderUpdate> {
    let id = job["requestId"].as_str().context("缺少 Codex 任务编号")?;
    let root = directory(&job["providerConfig"], id)?;
    let value: ProviderUpdate = serde_json::from_slice(&std::fs::read(root.join("state.json"))?)?;
    if matches!(value.status.as_str(), "IN_QUEUE" | "IN_PROGRESS")
        && !ACTIVE.lock().unwrap().contains_key(id)
    {
        return Ok(update(
            "FAILED",
            value.result,
            Some("本机 Codex 任务已中断，已完成的图片已保留".into()),
        ));
    }
    Ok(value)
}
async fn wait_for_slot<'a>(
    slots: &'a tokio::sync::Semaphore,
    cancel: &CancellationToken,
) -> Result<tokio::sync::SemaphorePermit<'a>> {
    tokio::select! {
        biased;
        _ = cancel.cancelled() => anyhow::bail!("任务已取消"),
        permit = slots.acquire() => Ok(permit?),
    }
}
async fn generate(
    input: &Value,
    root: &std::path::Path,
    cancel: CancellationToken,
) -> Result<Value> {
    codex_request::validate(input)?;
    phase(root, "connecting", "正在读取本机 Codex 登录")?;
    let credentials = tokio::select! {
        biased;
        _ = cancel.cancelled() => anyhow::bail!("任务已取消"),
        credentials = codex_auth::credentials() => credentials?,
    };
    let client = codex_http::client()?;
    let count = input["n"].as_u64().unwrap_or(1);
    let mut images = Vec::new();
    for index in 0..count {
        phase(
            root,
            "generating",
            &format!("已生成 {index}/{count} 张，正在直接生成图片"),
        )?;
        let result = tokio::select! {
            biased;
            _ = cancel.cancelled() => anyhow::bail!("任务已取消；已提交的服务端生成可能继续完成"),
            result = codex_http::request(&client, codex_http::BASE_URL, &credentials, input) => result?,
        };
        images.extend(result);
        persist(
            root,
            &update("IN_PROGRESS", Some(json!({"data":images})), None),
        )?;
    }
    Ok(json!({"data":images}))
}
impl ProviderAdapter for Codex {
    fn id(&self) -> &'static str {
        "codex-image"
    }
    fn validate_endpoint(&self, endpoint: &str) -> Result<()> {
        ensure!(
            endpoint == "codex://local/images",
            "Codex 使用固定本机生图连接"
        );
        Ok(())
    }
    fn submit<'a>(
        &'a self,
        _: &'a str,
        endpoint: &'a str,
        input: &'a Value,
        config: &'a Value,
        storage_lease: super::StorageLease,
    ) -> ProviderFuture<'a> {
        Box::pin(async move {
            self.validate_endpoint(endpoint)?;
            codex_request::validate(input)?;
            let id = mstudio::media::id();
            let root = directory(config, &id)?;
            std::fs::create_dir_all(&root)?;
            let cancel = CancellationToken::new();
            let mut initial = update("IN_QUEUE", None, None);
            initial.progress =
                Some(json!({"stage":"queued","message":"排队中，最多同时执行 2 个本机生图任务"}));
            initial.request_id = Some(id.clone());
            persist(&root, &initial)?;
            ACTIVE.lock().unwrap().insert(id.clone(), cancel.clone());
            let input = input.clone();
            tokio::spawn(async move {
                let _storage_lease = storage_lease;
                let result = async {
                    // Queue time must not consume the generation deadline.
                    let _permit = wait_for_slot(&SLOTS, &cancel).await?;
                    tokio::select! {
                        biased;
                        _ = cancel.cancelled() => Err(anyhow::anyhow!("任务已取消")),
                        result = tokio::time::timeout(Duration::from_secs(600), generate(&input, &root, cancel.clone())) =>
                            result.unwrap_or_else(|_| Err(anyhow::anyhow!("Codex 请求超时，已完成的图片已保留"))),
                    }
                }.await;
                let partial = std::fs::read(root.join("state.json"))
                    .ok()
                    .and_then(|s| serde_json::from_slice::<ProviderUpdate>(&s).ok())
                    .and_then(|s| s.result);
                let finished = if cancel.is_cancelled() {
                    update("CANCELLED", partial, None)
                } else {
                    match result {
                        Ok(value) => update("COMPLETED", Some(value), None),
                        Err(e) => update("FAILED", partial, Some(e.to_string())),
                    }
                };
                if let Err(e) = persist(&root, &finished) {
                    eprintln!("Codex 任务保存失败：{e}");
                }
                ACTIVE.lock().unwrap().remove(&id);
            });
            Ok(initial)
        })
    }
    fn refresh<'a>(&'a self, _: &'a str, job: &'a Value) -> ProviderFuture<'a> {
        Box::pin(async move { read(job) })
    }
    fn cancel<'a>(&'a self, _: &'a str, job: &'a Value) -> ProviderFuture<'a> {
        Box::pin(async move {
            let state = read(job)?;
            if matches!(state.status.as_str(), "IN_QUEUE" | "IN_PROGRESS") {
                if let Some(token) = ACTIVE
                    .lock()
                    .unwrap()
                    .get(job["requestId"].as_str().unwrap())
                {
                    token.cancel();
                }
                return Ok(update(
                    "IN_PROGRESS",
                    None,
                    Some("正在取消本地等待，服务端生成可能继续…".into()),
                ));
            }
            Ok(state)
        })
    }
    fn outputs(&self, value: &Value) -> Vec<ModelOutput> {
        value["data"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(|v| {
                v["b64_json"].as_str().map(|b| ModelOutput {
                    kind: "image".into(),
                    url: format!("data:image/png;base64,{b}"),
                })
            })
            .collect()
    }
}

#[cfg(test)]
#[path = "codex_tests.rs"]
mod tests;
