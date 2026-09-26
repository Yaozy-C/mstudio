//! Native MLT is the sole playback clock. IPC contains controls/status, never pixels.
use crate::database::Store;
use mstudio::model::RenderSpec;
use serde::Serialize;
use std::{
    ffi::{CStr, CString, c_char, c_void},
    sync::{Mutex, OnceLock},
};
use tauri::{Manager, WebviewWindow};
unsafe extern "C" {
    fn studio_player_open(
        parent: *mut c_void,
        root: *const c_char,
        xml: *const c_char,
        w: i32,
        h: i32,
        fps: i32,
    ) -> *const c_char;
    fn studio_player_rect(x: f64, y: f64, w: f64, h: f64, visible: i32);
    fn studio_player_play(playing: i32);
    fn studio_player_seek(frame: i32);
    fn studio_player_retire() -> *mut c_void;
    fn studio_player_stop_retired(handle: *mut c_void);
    fn studio_player_destroy_retired(handle: *mut c_void);
    fn studio_player_status(
        frame: *mut i32,
        playing: *mut i32,
        total: *mut i32,
        shown: *mut u64,
        skipped: *mut u64,
    );
}
static LIFECYCLE: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
static SESSION: OnceLock<Mutex<String>> = OnceLock::new();
fn session() -> &'static Mutex<String> {
    SESSION.get_or_init(|| Mutex::new(String::new()))
}
async fn on_main<T: Send + 'static>(
    window: WebviewWindow,
    work: impl FnOnce(WebviewWindow) -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    let (tx, rx) = tokio::sync::oneshot::channel();
    let copy = window.clone();
    window
        .run_on_main_thread(move || {
            let _ = tx.send(work(copy));
        })
        .map_err(|e| e.to_string())?;
    rx.await.map_err(|e| e.to_string())?
}
// Only the owner detached on the UI thread is touched by the blocking worker.
// Serialize retire/open so an old SDL shutdown cannot tear down a new device.
async fn retire(window: WebviewWindow) -> Result<(), String> {
    let handle = on_main(window.clone(), |_| {
        Ok(unsafe { studio_player_retire() } as usize)
    })
    .await?;
    if handle == 0 {
        return Ok(());
    }
    tauri::async_runtime::spawn_blocking(move || unsafe {
        studio_player_stop_retired(handle as *mut c_void);
    })
    .await
    .map_err(|e| e.to_string())?;
    on_main(window, move |_| {
        unsafe {
            studio_player_destroy_retired(handle as *mut c_void);
        }
        Ok(())
    })
    .await
}
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    frame: i32,
    playing: bool,
    total: i32,
    shown: u64,
    skipped: u64,
}
#[tauri::command]
pub async fn native_preview_open(
    window: WebviewWindow,
    app: tauri::AppHandle,
    project_id: String,
    token: String,
    spec: RenderSpec,
) -> Result<(), String> {
    *session().lock().unwrap() = token.clone();
    let _lifecycle = LIFECYCLE.lock().await;
    if *session().lock().unwrap() != token {
        return Err("预览请求已被替换".into());
    }
    retire(window.clone()).await?;
    let _guard = crate::project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut assets = app.state::<Store>().assets().map_err(|e| e.to_string())?;
    let audio_root = app.state::<Store>().media_root().join("proxies");
    let copy = app.clone();
    let (spec, xml) = tauri::async_runtime::spawn_blocking(move || -> Result<_, String> {
        static AUDIO_LOCK: Mutex<()> = Mutex::new(());
        let _lock = AUDIO_LOCK.lock().map_err(|e| e.to_string())?;
        let mut spec = spec;
        let files = mstudio::preview_audio::prepare(&mut spec, &mut assets, &audio_root)
            .map_err(|e| e.to_string())?;
        for path in files {
            crate::project_storage::track_file(&copy.state::<Store>(), &project_id, &path)
                .map_err(|e| e.to_string())?;
        }
        let xml = mstudio::preview_mlt::graph(&spec, &assets).map_err(|e| e.to_string())?;
        Ok((spec, xml))
    })
    .await
    .map_err(|e| e.to_string())??;
    let bundled = app
        .path()
        .resource_dir()
        .map_err(|e| e.to_string())?
        .join("mlt");
    let root = if bundled.join("lib/mlt").is_dir() {
        bundled
    } else {
        std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("native/runtime")
    };
    let xml = CString::new(xml).map_err(|e| e.to_string())?;
    let root = CString::new(root.to_string_lossy().as_bytes()).map_err(|e| e.to_string())?;
    on_main(window, move |window| {
        if *session().lock().unwrap() != token {
            return Err("预览请求已被替换".into());
        }
        #[cfg(target_os = "macos")]
        let parent = window.ns_window().map_err(|e| e.to_string())?;
        #[cfg(target_os = "windows")]
        let parent = window.hwnd().map_err(|e| e.to_string())?.0 as *mut c_void;
        let error = unsafe {
            studio_player_open(
                parent,
                root.as_ptr(),
                xml.as_ptr(),
                spec.width as i32,
                spec.height as i32,
                spec.fps as i32,
            )
        };
        if error.is_null() {
            Ok(())
        } else {
            Err(unsafe { CStr::from_ptr(error) }.to_string_lossy().into())
        }
    })
    .await
}
#[tauri::command]
pub async fn native_preview_control(
    window: WebviewWindow,
    token: String,
    action: String,
    frame: Option<i32>,
) -> Result<(), String> {
    if action == "close" {
        let _lifecycle = LIFECYCLE.lock().await;
        {
            let mut current = session().lock().unwrap();
            if *current != token {
                return Ok(());
            }
            current.clear();
        }
        return retire(window).await;
    }
    on_main(window, move |_| {
        if *session().lock().unwrap() != token {
            return Ok(());
        }
        unsafe {
            match action.as_str() {
                "play" => studio_player_play(1),
                "pause" => studio_player_play(0),
                "seek" => studio_player_seek(frame.unwrap_or(0)),
                _ => return Err("未知播放指令".into()),
            }
        }
        Ok(())
    })
    .await
}
#[tauri::command]
pub async fn native_preview_rect(
    window: WebviewWindow,
    token: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    visible: bool,
) -> Result<(), String> {
    if ![x, y, width, height].iter().all(|v| v.is_finite()) || width < 0. || height < 0. {
        return Err("预览区域无效".into());
    }
    on_main(window, move |_window| {
        if *session().lock().unwrap() != token {
            return Ok(());
        }
        #[cfg(target_os = "windows")]
        let scale = _window.scale_factor().map_err(|e| e.to_string())?;
        #[cfg(not(target_os = "windows"))]
        let scale = 1.;
        unsafe {
            studio_player_rect(
                x * scale,
                y * scale,
                width * scale,
                height * scale,
                visible as i32,
            )
        };
        Ok(())
    })
    .await
}
#[tauri::command]
pub async fn native_preview_status(window: WebviewWindow, token: String) -> Result<Status, String> {
    on_main(window, move |_| {
        if *session().lock().unwrap() != token {
            return Err("预览已关闭".into());
        }
        let mut s = Status::default();
        let mut playing = 0;
        unsafe {
            studio_player_status(
                &mut s.frame,
                &mut playing,
                &mut s.total,
                &mut s.shown,
                &mut s.skipped,
            )
        };
        s.playing = playing != 0;
        Ok(s)
    })
    .await
}
