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
        #[cfg(target_os = "macos")]
        player.detach_surface();
        tauri::async_runtime::spawn_blocking(move || player.shutdown())
            .await
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}
#[tauri::command]
pub async fn native_preview_open(
    app: tauri::AppHandle,
    window: tauri::WebviewWindow,
    project_id: String,
    token: String,
    spec: RenderSpec,
    edge: Option<u32>,
) -> Result<&'static str, String> {
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
    #[cfg(target_os = "macos")]
    let surface =
        crate::metal_preview::Surface::create(window, prepared.width, prepared.height).await?;
    #[cfg(not(target_os = "macos"))]
    let _ = window;
    let player = tauri::async_runtime::spawn_blocking(move || {
        PreviewProcess::launch(
            prepared,
            #[cfg(target_os = "macos")]
            surface,
        )
    })
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
    Ok(if cfg!(target_os = "macos") {
        "metal"
    } else {
        "canvas"
    })
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
#[tauri::command]
pub async fn native_preview_frame_info(
    token: String,
    last: u32,
) -> Result<Option<crate::preview_surface::FrameInfo>, String> {
    #[cfg(target_os = "macos")]
    return SESSION
        .lock()
        .unwrap()
        .get(&token)
        .ok_or("预览会话已关闭或被替换")?
        .frame_info(last);
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (token, last);
        Err("Metal 预览不可用".into())
    }
}
#[tauri::command]
pub async fn native_preview_present(
    token: String,
    sequence: u32,
    generation: u64,
) -> Result<bool, String> {
    #[cfg(target_os = "macos")]
    {
        let player = SESSION
            .lock()
            .unwrap()
            .get(&token)
            .ok_or("预览会话已关闭或被替换")?;
        tauri::async_runtime::spawn_blocking(move || player.present(sequence, generation))
            .await
            .map_err(|e| e.to_string())?
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (token, sequence, generation);
        Err("Metal 预览不可用".into())
    }
}
#[tauri::command]
pub async fn native_preview_bounds(
    token: String,
    bounds: crate::preview_surface::Bounds,
) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    {
        let surface = SESSION
            .lock()
            .unwrap()
            .get(&token)
            .and_then(|p| p.surface())
            .ok_or("预览会话已关闭或被替换")?;
        return surface.bounds(bounds).await;
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (token, bounds);
        Err("Metal 预览不可用".into())
    }
}
