//! GES owns the playback clock; the WebView consumes latest RGBA frames.
use crate::ges_engine::{Player, Status};
use mstudio::model::RenderSpec;
use std::sync::Mutex;
static PLAYER: Mutex<Option<Player>> = Mutex::new(None);
static LIFECYCLE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
struct Session {
    requested: String,
    active: String,
}
static SESSION: Mutex<Session> = Mutex::new(Session {
    requested: String::new(),
    active: String::new(),
});
async fn retire() -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(|| {
        let old = PLAYER.lock().unwrap().take();
        drop(old);
    })
    .await
    .map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn native_preview_open(
    app: tauri::AppHandle,
    project_id: String,
    token: String,
    spec: RenderSpec,
    edge: Option<u32>,
) -> Result<(), String> {
    let edge = edge.unwrap_or(640);
    if ![640, 1280].contains(&edge) {
        return Err("预览清晰度无效".into());
    }
    {
        let mut s = SESSION.lock().unwrap();
        s.requested = token.clone();
        s.active.clear();
    }
    let _lifecycle = LIFECYCLE.lock().await;
    if SESSION.lock().unwrap().requested != token {
        return Err("预览请求已被替换".into());
    }
    retire().await?;
    let prepared = crate::preview_prepare::prepare(app, project_id, spec, edge).await?;
    if SESSION.lock().unwrap().requested != token {
        return Err("预览请求已被替换".into());
    }
    let player = tauri::async_runtime::spawn_blocking(move || Player::open(prepared))
        .await
        .map_err(|e| e.to_string())??;
    *PLAYER.lock().unwrap() = Some(player);
    let mut s = SESSION.lock().unwrap();
    if s.requested != token {
        return Err("预览请求已被替换".into());
    }
    s.active = token;
    Ok(())
}
#[tauri::command]
pub async fn native_preview_control(
    token: String,
    action: String,
    frame: Option<i32>,
) -> Result<(), String> {
    if action == "close" {
        {
            let mut s = SESSION.lock().unwrap();
            if s.requested != token {
                return Ok(());
            }
            s.requested.clear();
            s.active.clear();
        }
        let _lifecycle = LIFECYCLE.lock().await;
        if !SESSION.lock().unwrap().requested.is_empty() {
            return Ok(());
        }
        return retire().await;
    }
    tauri::async_runtime::spawn_blocking(move || {
        let s = SESSION.lock().unwrap();
        if s.active != token {
            return Ok(());
        }
        PLAYER
            .lock()
            .unwrap()
            .as_ref()
            .ok_or("预览已关闭")?
            .control(&action, frame.unwrap_or(0))
            .map(|_| ())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn native_preview_status(token: String) -> Result<Status, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let s = SESSION.lock().unwrap();
        if s.active != token {
            return Err("预览已关闭".into());
        }
        PLAYER
            .lock()
            .unwrap()
            .as_ref()
            .ok_or("预览已关闭")?
            .control("status", 0)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn native_preview_frame(token: String, last: u32) -> tauri::ipc::Response {
    let s = SESSION.lock().unwrap();
    let data = if s.active == token {
        PLAYER
            .lock()
            .unwrap()
            .as_ref()
            .map(|p| p.frame(last))
            .unwrap_or_default()
    } else {
        vec![]
    };
    tauri::ipc::Response::new(data)
}
