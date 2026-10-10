use anyhow::{Result, ensure};
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};
use tokio_util::sync::CancellationToken;
static PENDING: LazyLock<Mutex<HashMap<String, CancellationToken>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));
pub(crate) struct PendingTurn {
    thread: String,
    pub token: CancellationToken,
}
impl PendingTurn {
    pub fn begin(thread: &str) -> Result<Self> {
        let mut pending = PENDING
            .lock()
            .map_err(|_| anyhow::anyhow!("对话状态异常"))?;
        ensure!(
            !pending.contains_key(thread),
            "此线程正在等待模型回答，请稍后再发送"
        );
        let token = CancellationToken::new();
        pending.insert(thread.into(), token.clone());
        Ok(Self {
            thread: thread.into(),
            token,
        })
    }
}
#[tauri::command]
pub(crate) fn cancel_assistant(thread_id: String) {
    super::parent_activation::close(&thread_id);
    if let Ok(pending) = PENDING.lock()
        && let Some(token) = pending.get(&thread_id)
    {
        token.cancel();
    }
}
impl Drop for PendingTurn {
    fn drop(&mut self) {
        if let Ok(mut pending) = PENDING.lock() {
            pending.remove(&self.thread);
        }
    }
}
