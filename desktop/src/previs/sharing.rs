use super::render::read_export;
use crate::{database::Store, project_service, project_storage};
use anyhow::{Context, Result, ensure};
use rusqlite::params;
use serde_json::{Value, json};
use std::collections::HashSet;
use tauri::Manager;

#[tauri::command]
pub async fn share_previs_video(
    app: tauri::AppHandle,
    project_id: String,
    id: String,
    shared: bool,
) -> Result<Value, String> {
    let guard = project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        set_shared(&app.state::<Store>(), &project_id, &id, shared).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
pub fn set_shared(store: &Store, project: &str, id: &str, shared: bool) -> Result<Value> {
    let mut export = {
        let db = store.db.lock().unwrap();
        read_export(&db, project, id)?
    };
    // Share a media-only copy. Removing the ordinary asset cannot delete the private render.
    let new_asset = if shared {
        let existing = {
            let db = store.db.lock().unwrap();
            let doc = project_service::load(&db, project)?;
            export.shared_asset_id.as_ref().is_some_and(|id| {
                doc["assets"]
                    .as_array()
                    .is_some_and(|a| a.iter().any(|v| v["id"] == *id))
            })
        };
        if existing {
            return Ok(json!({"shared":true,"assetId":export.shared_asset_id}));
        }
        let work = store
            .media_root()
            .join("imports")
            .join(format!("previs-share-{}", mstudio::media::id()));
        project_storage::track_file(store, project, &work)?;
        let asset = crate::imports::import_source_scoped(
            store,
            project,
            std::path::Path::new(&export.asset.path),
            &export.asset.name,
            &work,
            false,
        )?;
        let _ = crate::imports::discard(store, project, &work);
        Some(asset)
    } else {
        None
    };
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let before = project_service::load(&tx, project)?;
    let mut after = before.clone();
    if let Some(asset) = new_asset {
        let mut value = serde_json::to_value(&asset)?;
        value["inLibrary"] = json!(true);
        after["assets"]
            .as_array_mut()
            .context("素材目录无效")?
            .push(value);
        export.shared_asset_id = Some(asset.id);
    } else if let Some(asset_id) = &export.shared_asset_id {
        let mut usage = after.clone();
        usage["assets"] = json!([]);
        usage["removedAssetIds"] = json!([]);
        let mut refs = HashSet::new();
        project_storage::references(&usage, &mut refs);
        ensure!(
            !refs.contains(asset_id),
            "此视频正在被镜头、生成任务或时间线使用，请先解除引用再取消提供"
        );
        after["assets"]
            .as_array_mut()
            .context("素材目录无效")?
            .retain(|a| a["id"] != *asset_id);
        export.shared_asset_id = None;
    }
    if after != before {
        project_service::persist(&tx, &before, &after)?;
    }
    tx.execute(
        "UPDATE previs_exports SET data=?3 WHERE project_id=?1 AND id=?2",
        params![project, id, serde_json::to_string(&export)?],
    )?;
    tx.commit()?;
    drop(db);
    store.project_changed(project);
    Ok(json!({"shared":shared,"assetId":export.shared_asset_id}))
}
