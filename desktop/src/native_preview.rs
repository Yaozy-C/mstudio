//! Tauri adapter: session ownership, not engine synchronization.
use crate::{
    ges_engine::Status, preview_process::PreviewProcess, preview_protocol::Command,
    preview_session::Session,
};
use mstudio::model::RenderSpec;
use std::sync::{Arc, Mutex};
static SESSION: Mutex<Session<PreviewProcess>> = Mutex::new(Session::new());
// Cache writers stay serial; obsolete queued builds are skipped before doing work.
// This gate never guards controls, snapshots, or closing a worker.
static PREPARATION: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
async fn retire(player: Option<Arc<PreviewProcess>>) -> Result<(), String> {
    if let Some(player) = player {
        tauri::async_runtime::spawn_blocking(move || player.shutdown())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
pub async fn native_preview_open(
    app: tauri::AppHandle,
    project_id: String,
    token: String,
    spec: RenderSpec,
    edge: Option<u32>,
) -> Result<(), String> {
    let edge = edge.unwrap_or(0);
    if ![0, 640, 1280].contains(&edge) {
        return Err("预览清晰度无效".into());
    }
    let old = SESSION.lock().unwrap().begin(token.clone());
    retire(old).await?;
    if !SESSION.lock().unwrap().current(&token) {
        return Err("预览请求已被替换".into());
    }
    let prepared = {
        let _preparation = PREPARATION.lock().await;
        if !SESSION.lock().unwrap().current(&token) {
            return Err("预览请求已被替换".into());
        }
        crate::preview_prepare::prepare(app, project_id, spec, edge).await?
    };
    if !SESSION.lock().unwrap().current(&token) {
        return Err("预览请求已被替换".into());
    }
    let player = tauri::async_runtime::spawn_blocking(move || PreviewProcess::launch(prepared))
        .await
        .map_err(|e| e.to_string())??;
    let player = Arc::new(player);
    let rejected = SESSION
        .lock()
        .unwrap()
        .install(&token, player.clone())
        .err();
    if rejected.is_some() {
        retire(rejected).await?;
        return Err("预览请求已被替换".into());
    }
    tauri::async_runtime::spawn_blocking(move || player.ready())
        .await
        .map_err(|e| e.to_string())??;
    if !SESSION.lock().unwrap().current(&token) {
        return Err("预览请求已被替换".into());
    }
    Ok(())
}
#[tauri::command]
pub async fn native_preview_control(token: String, command: Command) -> Result<Status, String> {
    if matches!(command, Command::Close) {
        let old = SESSION.lock().unwrap().close(&token);
        retire(old).await?;
        return Ok(Status::default());
    }
    let player = SESSION
        .lock()
        .unwrap()
        .get(&token)
        .ok_or("预览会话已关闭或被替换")?;
    tauri::async_runtime::spawn_blocking(move || player.control(command))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn native_preview_status(token: String) -> Result<Status, String> {
    let player = SESSION
        .lock()
        .unwrap()
        .get(&token)
        .ok_or("预览会话已关闭或被替换")?;
    player.snapshot()
}
#[tauri::command]
pub async fn native_preview_frame(token: String, last: u32) -> tauri::ipc::Response {
    let player = SESSION.lock().unwrap().get(&token);
    tauri::ipc::Response::new(player.map(|p| p.frame(last)).unwrap_or_default())
}
