use crate::database::Store;
use anyhow::Result;
use mstudio::model::Asset;
use rusqlite::{Connection, params};
use std::collections::HashSet;
use tauri::State;

pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS global_assets(
        asset_id TEXT PRIMARY KEY REFERENCES assets(id), added INTEGER NOT NULL);
        UPDATE assets SET data=json_set(data,'$.generated',json('true'))
        WHERE coalesce(json_extract(data,'$.generated'),0)=0
        AND id IN (SELECT json_extract(data,'$.asset.id') FROM jobs);",
    )?;
    Ok(())
}

pub fn generated_ids(db: &Connection) -> Result<HashSet<String>> {
    let mut stmt = db.prepare("SELECT id FROM assets WHERE json_extract(data,'$.generated')=1")?;
    Ok(stmt
        .query_map([], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

pub fn list(db: &Connection) -> Result<Vec<Asset>> {
    let mut stmt = db.prepare("SELECT a.data FROM assets a JOIN global_assets g ON a.id=g.asset_id ORDER BY g.added DESC,g.asset_id")?;
    let rows = stmt
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.iter()
        .map(|raw| {
            let mut asset: Asset = serde_json::from_str(raw)?;
            asset.missing = !std::path::Path::new(&asset.path).is_file();
            Ok(asset)
        })
        .collect()
}

pub fn promote(db: &Connection, id: &str) -> Result<()> {
    let changed = db.execute("INSERT INTO global_assets SELECT id,unixepoch() FROM assets WHERE id=?1 ON CONFLICT(asset_id) DO NOTHING", [id])?;
    anyhow::ensure!(
        changed > 0
            || db.query_row(
                "SELECT EXISTS(SELECT 1 FROM global_assets WHERE asset_id=?1)",
                [id],
                |r| r.get::<_, bool>(0)
            )?,
        "素材不存在"
    );
    Ok(())
}

pub fn save_global(store: &Store, asset: &Asset) -> Result<()> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    tx.execute(
        "INSERT INTO assets VALUES(?1,?2)",
        params![asset.id, serde_json::to_string(asset)?],
    )?;
    promote(&tx, &asset.id)?;
    tx.commit()?;
    Ok(())
}

pub fn attach(db: &Connection, project: &str, id: &str) -> Result<Asset> {
    let raw: String = db.query_row(
        "SELECT a.data FROM assets a JOIN global_assets g ON a.id=g.asset_id WHERE a.id=?1",
        [id],
        |r| r.get(0),
    )?;
    let mut asset: Asset = serde_json::from_str(&raw)?;
    asset.missing = !std::path::Path::new(&asset.path).is_file();
    db.execute(
        "INSERT OR IGNORE INTO project_assets VALUES(?1,?2)",
        params![project, id],
    )?;
    Ok(asset)
}

#[tauri::command]
pub fn list_global_assets(store: State<Store>) -> Result<Vec<Asset>, String> {
    list(&store.db.lock().unwrap()).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn add_global_asset(store: State<'_, Store>, id: String) -> Result<(), String> {
    let _guard = store.files.clone().read_owned().await;
    promote(&store.db.lock().unwrap(), &id).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn remove_global_asset(store: State<'_, Store>, id: String) -> Result<(), String> {
    let _guard = store.files.clone().read_owned().await;
    // Remove library membership only: project usages and their files remain valid.
    store
        .db
        .lock()
        .unwrap()
        .execute("DELETE FROM global_assets WHERE asset_id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
pub async fn use_global_asset(
    store: State<'_, Store>,
    project_id: String,
    id: String,
) -> Result<Asset, String> {
    let _guard = crate::project_storage::working(&store, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    attach(&store.db.lock().unwrap(), &project_id, &id).map_err(|e| e.to_string())
}
