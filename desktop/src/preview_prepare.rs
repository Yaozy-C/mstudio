use crate::database::Store;
use mstudio::model::RenderSpec;
use tauri::Manager;
pub async fn prepare(
    app: tauri::AppHandle,
    project_id: String,
    spec: RenderSpec,
    edge: u32,
) -> Result<mstudio::preview_ges::Plan, String> {
    let guard = crate::project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let store = app.state::<Store>();
        let mut assets = store.assets().map_err(|e| e.to_string())?;
        let root = store.media_root().join("proxies");
        let mut spec = spec;
        mstudio::preview_validate::validate(&spec, &assets, edge).map_err(|e| e.to_string())?;
        let mut files = mstudio::transitions::prepare(&mut spec, &mut assets, &root, true)
            .map_err(|e| e.to_string())?;
        let result = mstudio::preview_ges::prepare(&spec, &assets, &root, edge)
            .map_err(|e| e.to_string())?;
        files.extend(result.files.iter().cloned());
        for path in files {
            crate::project_storage::track_file(&store, &project_id, &path)
                .map_err(|e| e.to_string())?;
        }
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}
