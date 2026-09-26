use crate::database::Store;
use mstudio::{audio_mix, model::RenderSpec};
use std::{
    hash::{Hash, Hasher},
    sync::Mutex,
};
use tauri::Manager;
static MIX_LOCK: Mutex<()> = Mutex::new(());
#[tauri::command]
pub async fn prepare_audio_preview(
    app: tauri::AppHandle,
    project_id: String,
    spec: RenderSpec,
) -> Result<Option<String>, String> {
    let guard = crate::project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _project_guard = guard;
        let _guard = MIX_LOCK.lock().map_err(|e| e.to_string())?;
        let store = app.state::<Store>();
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        serde_json::to_string(&spec)
            .map_err(|e| e.to_string())?
            .hash(&mut hash);
        let folder = store.media_root().join("proxies");
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let path = folder.join(format!("audio-v1-{:x}.m4a", hash.finish()));
        crate::project_storage::track_file(&store, &project_id, &path)
            .map_err(|e| e.to_string())?;
        if path.is_file() {
            return Ok(Some(path.to_string_lossy().into()));
        }
        let temporary = folder.join(format!("mix-{}.m4a", mstudio::media::id()));
        crate::project_storage::track_file(&store, &project_id, &temporary)
            .map_err(|e| e.to_string())?;
        let assets = store.assets().map_err(|e| e.to_string())?;
        match audio_mix::preview(&spec, &assets, &temporary) {
            Ok(true) => {
                std::fs::rename(temporary, &path).map_err(|e| e.to_string())?;
                Ok(Some(path.to_string_lossy().into()))
            }
            Ok(false) => Ok(None),
            Err(e) => {
                let _ = std::fs::remove_file(temporary);
                Err(e.to_string())
            }
        }
    })
    .await
    .map_err(|e| e.to_string())?
}
