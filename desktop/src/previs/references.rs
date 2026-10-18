//! Reference the private render directly; only an explicit save imports library media.
use super::{export_store::read_export, media};
use crate::{database::Store, project_service, project_storage};
use anyhow::{Context, Result, ensure};
use rusqlite::params;
use serde_json::{Value, json};
use tauri::Manager;

#[tauri::command]
pub async fn attach_previs_reference(
    app: tauri::AppHandle,
    project_id: String,
    file_id: String,
    id: String,
    asset_id: String,
) -> Result<(), String> {
    let store = app.state::<Store>();
    let guard = project_storage::working(&store, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        attach(&app.state::<Store>(), &project_id, &file_id, &id, &asset_id)
            .map_err(|e| e.to_string())?;
        project_service::notify(&app, &project_id);
        Ok(())
    })
    .await
    .map_err(|e| e.to_string())?
}
pub fn attach(store: &Store, project: &str, file: &str, id: &str, asset_id: &str) -> Result<()> {
    let _guard = super::export_store::WRITE.lock().unwrap();
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let export = read_export(&tx, project, id)?;
    ensure!(
        export.asset.id == asset_id,
        "预演已重新生成，请刷新后再设为参考"
    );
    ensure!(
        std::path::Path::new(&export.asset.path).is_file(),
        "预演视频不可用，请重新生成"
    );
    media::register(&tx, project, &export.asset)?;
    let before = project_service::load(&tx, project)?;
    let mut after = before.clone();
    let node = after["nodes"]
        .as_array_mut()
        .and_then(|nodes| {
            nodes
                .iter_mut()
                .find(|n| n["id"] == file && n["kind"] == "shot")
        })
        .context("镜头已删除")?;
    if !node["references"].is_array() {
        node["references"] = json!([]);
    }
    let refs = node["references"].as_array_mut().unwrap();
    if !refs.iter().any(|r| r["assetId"] == asset_id) {
        refs.push(json!({"assetId":asset_id,"purpose":"预演动作、构图与运镜参考；商品外观以实物参考为准","start":0.,"end":export.asset.duration}));
    }
    if before != after {
        project_service::persist(&tx, &before, &after)?;
    }
    tx.commit()?;
    drop(db);
    store.project_changed(project);
    Ok(())
}
#[tauri::command]
pub async fn collect_previs_video(
    app: tauri::AppHandle,
    project_id: String,
    id: String,
    asset_id: String,
) -> Result<Value, String> {
    let store = app.state::<Store>();
    let guard = project_storage::working(&store, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let asset = collect(&app.state::<Store>(), &project_id, &id, &asset_id)
            .map_err(|e| e.to_string())?;
        project_service::notify(&app, &project_id);
        Ok(asset)
    })
    .await
    .map_err(|e| e.to_string())?
}
pub fn collect(store: &Store, project: &str, id: &str, asset_id: &str) -> Result<Value> {
    let _guard = super::export_store::WRITE.lock().unwrap();
    let mut export = read_export(&store.db.lock().unwrap(), project, id)?;
    ensure!(
        export.asset.id == asset_id,
        "预演已重新生成，请刷新后再保存"
    );
    let document = project_service::load(&store.db.lock().unwrap(), project)?;
    let existing = export
        .saved_asset_id
        .as_ref()
        .and_then(|id| {
            document["assets"].as_array()?.iter().find(|a| {
                a["id"] == *id
                    && a["discarded"] != true
                    && a["path"]
                        .as_str()
                        .is_some_and(|p| std::path::Path::new(p).is_file())
            })
        })
        .cloned();
    let mut asset = if let Some(value) = existing {
        value
    } else {
        let work = store
            .media_root()
            .join("imports")
            .join(format!("previs-save-{}", mstudio::media::id()));
        project_storage::track_file(store, project, &work)?;
        let name = format!("{}.mp4", export.asset.name.trim_end_matches(".mp4"));
        let result = crate::imports::import_source_scoped(
            store,
            project,
            std::path::Path::new(&export.asset.path),
            &name,
            &work,
            false,
        );
        let _ = crate::imports::discard(store, project, &work);
        serde_json::to_value(result?)?
    };
    asset["generated"] = json!(true);
    asset["inLibrary"] = json!(true);
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let before = project_service::load(&tx, project)?;
    let mut after = before.clone();
    let assets = after["assets"].as_array_mut().context("素材目录无效")?;
    if let Some(current) = assets.iter_mut().find(|a| a["id"] == asset["id"]) {
        *current = asset.clone();
    } else {
        assets.push(asset.clone());
    }
    export.saved_asset_id = Some(asset["id"].as_str().context("素材 ID 缺失")?.into());
    let mut metadata = asset.clone();
    metadata.as_object_mut().unwrap().remove("inLibrary");
    tx.execute(
        "UPDATE assets SET data=?2 WHERE id=?1",
        params![asset["id"].as_str(), metadata.to_string()],
    )?;
    tx.execute(
        "UPDATE previs_exports SET data=?3 WHERE project_id=?1 AND id=?2",
        params![project, id, serde_json::to_string(&export)?],
    )?;
    project_service::persist(&tx, &before, &after)?;
    tx.commit()?;
    drop(db);
    store.project_changed(project);
    Ok(asset)
}
