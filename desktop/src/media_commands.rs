use crate::database::Store;
use mstudio::{media, model::RenderSpec};
use serde_json::{Value, json};
use tauri::{Emitter, Manager};

#[tauri::command]
pub async fn video_source_ends(
    app: tauri::AppHandle,
    project_id: String,
    asset_ids: Vec<String>,
) -> Result<std::collections::HashMap<String, f64>, String> {
    let guard = crate::project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let store = app.state::<Store>();
        let project = crate::project_service::load(&store.db.lock().unwrap(), &project_id)
            .map_err(|e| e.to_string())?;
        let assets: Vec<mstudio::model::Asset> =
            serde_json::from_value(project["assets"].clone()).map_err(|e| e.to_string())?;
        let mut ends = std::collections::HashMap::new();
        for id in asset_ids {
            if ends.contains_key(&id) {
                continue;
            }
            let asset = assets.iter().find(|a| a.id == id).ok_or("素材不存在")?;
            if asset.kind != "video" || asset.missing {
                return Err(format!("无法读取视频：{}", asset.name));
            }
            let metadata = media::probe(std::path::Path::new(&asset.path))
                .map_err(|e| format!("{}：{e}", asset.name))?;
            let end = mstudio::media_timing::video_end(&metadata)
                .map_err(|e| format!("{}：{e}", asset.name))?;
            ends.insert(id, end);
        }
        Ok(ends)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub async fn render_video(
    app: tauri::AppHandle,
    project_id: String,
    spec: RenderSpec,
) -> Result<Value, String> {
    let guard = crate::project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let store = app.state::<Store>();
        let id = media::id();
        let work = store.media_root().join("render-work").join(&id);
        let exports = store.media_root().join("exports");
        std::fs::create_dir_all(&exports).map_err(|e| e.to_string())?;
        let output = exports.join(format!("mstudio-{id}.mp4"));
        crate::project_storage::track_file(&store, &project_id, &work).map_err(|e| e.to_string())?;
        crate::project_storage::track_file(&store, &project_id, &output).map_err(|e| e.to_string())?;
        let assets = store.assets().map_err(|e| e.to_string())?;
        if let Some(asset) = assets.iter().find(|a| a.missing && spec.clips.iter().any(|c| c.asset_id == a.id)) {
            return Err(format!("文件缺失：{}。请恢复文件后再导出。", asset.name));
        }
        let result = mstudio::render::render(&spec, &assets, &work, &output, |done, total| { let _ = app.emit("render-progress", json!({"done":done,"total":total})); });
        let _ = std::fs::remove_dir_all(work);
        result.map_err(|e| e.to_string())?;
        Ok(json!({"path":output.to_string_lossy(),"name":output.file_name().unwrap().to_string_lossy()}))
    }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn save_export(app: tauri::AppHandle, path: String) -> Result<Option<String>, String> {
    let root = app
        .state::<Store>()
        .media_root()
        .join("exports")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let source = std::path::PathBuf::from(path)
        .canonicalize()
        .map_err(|e| e.to_string())?;
    if !source.starts_with(root) {
        return Err("只能导出应用生成的成片".into());
    }
    let file = rfd::AsyncFileDialog::new()
        .set_file_name("mstudio-film.mp4")
        .add_filter("MP4", &["mp4"])
        .save_file()
        .await;
    if let Some(file) = file {
        if file.path() != source {
            std::fs::copy(source, file.path()).map_err(|e| e.to_string())?;
        }
        return Ok(Some(file.path().to_string_lossy().into()));
    }
    Ok(None)
}

#[tauri::command]
pub async fn reveal_export(path: String) -> Result<(), String> {
    tauri::async_runtime::spawn_blocking(move || {
        let file = std::path::PathBuf::from(path)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !file.is_file() {
            return Err("成片文件不存在".into());
        }
        #[cfg(target_os = "macos")]
        let status = std::process::Command::new("open")
            .arg("-R")
            .arg(&file)
            .status();
        #[cfg(target_os = "windows")]
        let status = std::process::Command::new("explorer")
            .arg(format!("/select,{}", file.display()))
            .status();
        #[cfg(not(any(target_os = "macos", target_os = "windows")))]
        let status = std::process::Command::new("xdg-open")
            .arg(file.parent().ok_or("无法读取保存位置")?)
            .status();
        if !status.map_err(|e| e.to_string())?.success() {
            return Err("无法打开文件夹".into());
        }
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
