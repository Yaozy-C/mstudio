mod cleanup;
mod plan;
#[cfg(test)]
mod tests;

use crate::database::Store;
use anyhow::{Result, ensure};
use mstudio::model::Asset;
use rusqlite::{Connection, params};
use serde_json::Value;
use std::{collections::HashSet, path::Path};
use tokio::sync::OwnedRwLockReadGuard;

pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS project_assets(
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        asset_id TEXT NOT NULL, PRIMARY KEY(project_id,asset_id));
        CREATE TABLE IF NOT EXISTS project_files(
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        path TEXT NOT NULL, PRIMARY KEY(project_id,path));
        CREATE TABLE IF NOT EXISTS pending_file_deletions(
        project_id TEXT NOT NULL,path TEXT NOT NULL,PRIMARY KEY(project_id,path));",
    )?;
    Ok(())
}

pub async fn working(store: &Store, id: &str) -> Result<OwnedRwLockReadGuard<()>> {
    let guard = store.files.clone().read_owned().await;
    ensure!(
        store.media_root().is_dir(),
        "存储目录不可用，请检查存储设备或设置"
    );
    ensure!(
        store.db.lock().unwrap().query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)",
            [id],
            |r| r.get::<_, bool>(0)
        )?,
        "项目已删除"
    );
    Ok(guard)
}

// Retain ownership even when a user removes an asset from the canvas/library.
pub fn remember(db: &Connection, id: &str, document: &Value) -> Result<()> {
    let mut refs = HashSet::new();
    references(document, &mut refs);
    for asset in refs {
        db.execute(
            "INSERT OR IGNORE INTO project_assets VALUES(?1,?2)",
            params![id, asset],
        )?;
    }
    Ok(())
}

pub fn references(value: &Value, ids: &mut HashSet<String>) {
    match value {
        Value::Object(object) => {
            for key in ["assetId", "resultAssetId"] {
                if let Some(id) = object.get(key).and_then(Value::as_str) {
                    ids.insert(id.into());
                }
            }
            if (object.contains_key("path") || object.get("kind").is_some_and(|v| v == "asset"))
                && let Some(id) = object.get("id").and_then(Value::as_str)
            {
                ids.insert(id.into());
            }
            for child in object.values() {
                references(child, ids);
            }
        }
        Value::Array(array) => {
            for child in array {
                references(child, ids);
            }
        }
        _ => {}
    }
}

pub fn save_asset(store: &Store, id: &str, asset: &Asset) -> Result<()> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO project_assets VALUES(?1,?2)",
        params![id, asset.id],
    )?;
    tx.execute(
        "INSERT INTO assets VALUES(?1,?2)",
        params![asset.id, serde_json::to_string(asset)?],
    )?;
    tx.commit()?;
    Ok(())
}

pub fn track_file(store: &Store, id: &str, path: &Path) -> Result<()> {
    store.db.lock().unwrap().execute(
        "INSERT OR IGNORE INTO project_files VALUES(?1,?2)",
        params![id, path.to_string_lossy()],
    )?;
    Ok(())
}

pub fn remove(store: &Store, id: &str) -> Result<()> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let plan = plan::build(&tx, &store.media_root(), id)?;
    // The cleanup queue commits with the records, so a crash or a locked file is retryable.
    for path in plan.files {
        tx.execute(
            "INSERT OR IGNORE INTO pending_file_deletions VALUES(?1,?2)",
            params![id, path.to_string_lossy()],
        )?;
    }
    tx.execute("DELETE FROM jobs WHERE project_id=?1", [id])?;
    tx.execute("DELETE FROM projects WHERE id=?1", [id])?;
    for asset in plan.assets {
        tx.execute("DELETE FROM assets WHERE id=?1", [asset])?;
    }
    tx.commit()?;
    cleanup::run(&db, &store.media_root(), Some(id))
}

pub fn resume_cleanup(db: &Connection, root: &Path) -> Result<()> {
    cleanup::run(db, root, None)
}
