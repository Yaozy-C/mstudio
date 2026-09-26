use super::{
    ModelOutput, ProviderAdapter, ProviderFuture, ProviderUpdate, codex_request, codex_rpc::Rpc,
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
    if value.status == "IN_PROGRESS" && !ACTIVE.lock().unwrap().contains_key(id) {
        return Ok(update(
            "FAILED",
            value.result,
            Some("本机 Codex 任务已中断，已完成的图片已保留".into()),
        ));
    }
    Ok(value)
}
async fn generate(
    input: &Value,
    root: &std::path::Path,
    cancel: CancellationToken,
) -> Result<Value> {
    phase(root, "connecting", "正在连接本机 Codex")?;
    let items = codex_request::turn_input(input, root)?;
    let mut rpc = Rpc::start().await?;
    let result: Result<Value> = async {
        let account = rpc.call("account/read", json!({"refreshToken":false})).await?;
        ensure!(account["account"]["type"] == "chatgpt", "请先在本机 Codex 登录 ChatGPT 账号");
        let capabilities = rpc.call("modelProvider/capabilities/read", json!({})).await?;
        ensure!(capabilities["imageGeneration"] == true, "当前 Codex 不支持原生生图");
        phase(root, "preparing", "Codex 已连接，正在准备参考素材")?;
        let started = rpc.call("thread/start", json!({"cwd":root,"ephemeral":true,"approvalPolicy":"never","sandbox":"workspace-write","developerInstructions":"This is an image-only request. Use native imagegen. Do not run shell commands, modify project files, or use other providers.","config":{"features":{"image_generation":true}}})).await?;
        let thread = started["thread"]["id"].as_str().context("Codex 未返回会话编号")?.to_string();
        let started = rpc.call("turn/start", json!({"threadId":thread,"input":items})).await?;
        let turn = started["turn"]["id"].as_str().context("Codex 未返回回合编号")?.to_string();
        phase(root, "submitted", "请求已交给 Codex，等待调用生图工具")?;
        let mut images = Vec::new();
        loop {
            let event = tokio::select! {
                _ = cancel.cancelled() => {
                    let _ = tokio::time::timeout(Duration::from_secs(3), rpc.call("turn/interrupt", json!({"threadId":thread,"turnId":turn}))).await;
                    anyhow::bail!("任务已取消");
                }
                event = rpc.next() => event?
            };
            if event["method"] == "item/started" && event["params"]["item"]["type"] == "imageGeneration" {
                phase(root, "generating", &format!("已生成 {} 张，正在生成图片", images.len()))?;
            }
            if event["method"] == "item/completed" && event["params"]["item"]["type"] == "imageGeneration" {
                if let Some(image) = codex_request::image_result(&event["params"]["item"])? {
                    images.push(image);
                    persist(root, &update("IN_PROGRESS", Some(json!({"data":images})), None))?;
                    phase(root, "generated", &format!("已生成 {} 张图片", images.len()))?;
                } else {
                    phase(root, "generating", "本次生图未成功，模型正在处理")?;
                }
            }
            if event["method"] == "turn/completed" {
                let completed = &event["params"]["turn"];
                ensure!(completed["status"] == "completed", "Codex 回合失败：{}", completed["error"]);
                ensure!(!images.is_empty(), "Codex 未返回图片");
                return Ok(json!({"data":images}));
            }

            rpc.decline(&event).await?;
        }
    }.await;
    rpc.stop().await;
    result
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
            let mut initial = update("IN_PROGRESS", None, None);
            initial.request_id = Some(id.clone());
            persist(&root, &initial)?;
            ACTIVE.lock().unwrap().insert(id.clone(), cancel.clone());
            let input = input.clone();
            tokio::spawn(async move {
                let _storage_lease = storage_lease;
                let result = tokio::time::timeout(
                    Duration::from_secs(600),
                    generate(&input, &root, cancel.clone()),
                )
                .await
                .unwrap_or_else(|_| Err(anyhow::anyhow!("Codex 生图超时，已完成的图片已保留")));
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
            if state.status == "IN_PROGRESS" {
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
                    Some("正在取消本机生图…".into()),
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
