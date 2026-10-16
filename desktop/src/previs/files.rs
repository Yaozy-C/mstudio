use super::File;
use crate::{database::Store, project_service, project_storage};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::path::PathBuf;
use tauri::Manager;

pub fn read(db: &Connection, project: &str, id: &str) -> Result<Option<File>> {
    db.query_row(
        "SELECT data FROM previs_files WHERE project_id=?1 AND id=?2",
        params![project, id],
        |r| r.get::<_, String>(0),
    )
    .optional()?
    .map(|s| serde_json::from_str(&s).map_err(Into::into))
    .transpose()
}
pub fn list(db: &Connection, project: &str) -> Result<Vec<File>> {
    db.prepare("SELECT data FROM previs_files WHERE project_id=?1 ORDER BY kind,json_extract(data,'$.name') COLLATE NOCASE")?
        .query_map([project], |r| r.get::<_, String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?
        .iter().map(|s| serde_json::from_str(s).map_err(Into::into)).collect()
}
pub fn path(store: &Store, project: &str, file: &File) -> Result<PathBuf> {
    super::identifier(project)?;
    super::identifier(&file.id)?;
    Ok(store
        .media_root()
        .join("previs")
        .join(project)
        .join(&file.id)
        .join(if file.kind == "model" {
            "model.js"
        } else {
            "scene.js"
        }))
}
pub fn validate_target(db: &Connection, project: &str, file: &File) -> Result<()> {
    file.validate()?;
    let doc = project_service::load(db, project)?;
    if file.kind == "shot" {
        ensure!(
            doc["nodes"]
                .as_array()
                .is_some_and(|ns| ns.iter().any(|n| n["id"] == file.id && n["kind"] == "shot")),
            "镜头已删除"
        );
    }
    let all = list(db, project)?;
    if file.kind == "model" {
        ensure!(
            file.asset_ids.is_empty() && file.bindings.is_empty(),
            "共用模型只定义自身结构，模型组装属于镜头"
        );
        ensure!(
            !all.iter().any(|m| m.kind == "model"
                && m.id != file.id
                && m.name.trim().to_lowercase() == file.name.trim().to_lowercase()),
            "项目已有同名模型，请复用或换一个名称"
        );
    }
    for id in &file.asset_ids {
        ensure!(
            all.iter().any(|m| m.id == *id && m.kind == "model"),
            "镜头引用的模型已删除：{id}"
        );
    }
    ensure!(
        file.bindings.values().all(|id| file.asset_ids.contains(id)),
        "模型名称绑定须属于镜头引用的资产"
    );
    Ok(())
}
pub fn model_in_use(db: &Connection, project: &str, id: &str) -> Result<bool> {
    let doc = project_service::load(db, project)?;
    Ok(list(db, project)?.iter().any(|file| {
        file.kind == "shot"
            && file.asset_ids.iter().any(|asset| asset == id)
            && doc["nodes"].as_array().is_some_and(|nodes| {
                nodes
                    .iter()
                    .any(|node| node["id"] == file.id && node["kind"] == "shot")
            })
    }))
}
/// Caller holds the DB transaction. Each source has one stable, replaceable path.
/// Runtime snapshots use the committed DB content, independent of this source mirror.
pub fn save(
    db: &Connection,
    store: &Store,
    project: &str,
    mut file: File,
    expected: u64,
) -> Result<File> {
    validate_target(db, project, &file)?;
    let old = read(db, project, &file.id)?;
    ensure!(
        old.as_ref().map_or(0, |f| f.revision) == expected,
        "文件已被修改，请刷新后重新修改"
    );
    ensure!(
        old.as_ref().is_none_or(|f| f.kind == file.kind),
        "不能更改预演文件类型"
    );
    file.name = file.name.trim().into();
    file.revision = expected + 1;
    file.asset_ids.sort();
    file.asset_ids.dedup();
    let target = path(store, project, &file)?;
    let directory = target.parent().context("预演目录无效")?;
    db.execute(
        "INSERT OR IGNORE INTO project_files VALUES(?1,?2)",
        params![
            project,
            store
                .media_root()
                .join("previs")
                .join(project)
                .to_string_lossy()
        ],
    )?;
    std::fs::create_dir_all(directory)?;
    let temporary = directory.join(format!(".{}.tmp", uuid::Uuid::new_v4()));
    let write = (|| -> std::io::Result<()> {
        std::fs::write(&temporary, &file.code)?;
        std::fs::rename(&temporary, &target)
    })();
    if write.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    write?;
    db.execute("INSERT INTO previs_files VALUES(?1,?2,?3,?4) ON CONFLICT(project_id,id) DO UPDATE SET data=excluded.data", params![project,file.id,file.kind,serde_json::to_string(&file)?])?;
    Ok(file)
}
#[tauri::command]
pub async fn save_previs_file(
    app: tauri::AppHandle,
    project_id: String,
    file: File,
    expected_revision: u64,
) -> Result<File, String> {
    let store = app.state::<Store>();
    let _guard = project_storage::working(&store, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let saved =
        save(&tx, &store, &project_id, file, expected_revision).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(saved)
}
#[tauri::command]
pub async fn previs_workspace(app: tauri::AppHandle, project_id: String) -> Result<Value, String> {
    let guard = project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
    let _guard = guard;
    let store = app.state::<Store>();
    (|| -> Result<Value> {
        if super::export_store::normalize(&store, &project_id)? {
            project_service::notify(&app, &project_id);
        }
        let db = store.db.lock().unwrap();
        project_service::load(&db, &project_id)?;
        let files = list(&db, &project_id)?;
        let messages = db.prepare("SELECT file_id,role,text FROM previs_messages WHERE project_id=?1 ORDER BY id DESC LIMIT 200")?
            .query_map([&project_id], |r| Ok(json!({"fileId":r.get::<_,String>(0)?,"role":r.get::<_,String>(1)?,"text":r.get::<_,String>(2)?})))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        let exports = db.prepare("SELECT data FROM previs_exports WHERE project_id=?1 ORDER BY rowid DESC LIMIT 100")?
            .query_map([&project_id], |r| r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?
            .iter().map(|s| serde_json::from_str::<Value>(s)).collect::<serde_json::Result<Vec<_>>>()?;
        Ok(json!({"files":files,"messages":messages.into_iter().rev().collect::<Vec<_>>(),"exports":exports}))
    })().map_err(|e| e.to_string())
    }).await.map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn delete_previs_model(
    app: tauri::AppHandle,
    project_id: String,
    id: String,
    expected_revision: u64,
) -> Result<(), String> {
    let store = app.state::<Store>();
    let _guard = project_storage::working(&store, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    (|| -> Result<()> {
        let db = store.db.lock().unwrap();
        let file = read(&db, &project_id, &id)?.context("模型已删除")?;
        ensure!(
            file.kind == "model" && file.revision == expected_revision,
            "模型已被修改，请刷新"
        );
        ensure!(
            !model_in_use(&db, &project_id, &id)?,
            "模型仍被镜头引用，请先解除引用"
        );
        db.execute(
            "DELETE FROM previs_files WHERE project_id=?1 AND id=?2",
            params![project_id, id],
        )?;
        Ok(())
    })()
    .map_err(|e| e.to_string())
}
