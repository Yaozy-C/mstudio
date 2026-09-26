use crate::database::Store;
use std::sync::{Mutex, OnceLock};
use tauri::Manager;
// Avoid competing transcodes and duplicate cache writes across project panels.
static PROXY_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
#[tauri::command]
pub async fn prepare_preview(
    app: tauri::AppHandle,
    asset_id: String,
    project_id: String,
) -> Result<String, String> {
    let guard = crate::project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _project_guard = guard;
        let _guard = PROXY_LOCK
            .get_or_init(|| Mutex::new(()))
            .lock()
            .map_err(|_| "预览队列异常")?;
        let store = app.state::<Store>();
        let asset = store
            .assets()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|a| a.id == asset_id)
            .ok_or("素材不存在")?;
        if asset.missing {
            return Err(format!("文件缺失：{}", asset.name));
        }
        let path =
            mstudio::proxy::prepare(&asset, &store.media_root()).map_err(|e| e.to_string())?;
        crate::project_storage::track_file(&store, &project_id, &path)
            .map_err(|e| e.to_string())?;
        Ok(path.to_string_lossy().into())
    })
    .await
    .map_err(|e| e.to_string())?
}
