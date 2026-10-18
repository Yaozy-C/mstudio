//! Immutable previsualization media; never ordinary asset-library membership.
use anyhow::Result;
use mstudio::model::Asset;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::collections::HashSet;

pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS previs_media(project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,id TEXT NOT NULL,data TEXT NOT NULL,PRIMARY KEY(project_id,id));")?;
    Ok(())
}
pub fn register(db: &Connection, project: &str, asset: &Asset) -> Result<()> {
    db.execute(
        "INSERT OR IGNORE INTO previs_media VALUES(?1,?2,?3)",
        params![project, asset.id, serde_json::to_string(asset)?],
    )?;
    Ok(())
}
pub fn hydrate(db: &Connection, project: &str, document: &mut Value) -> Result<()> {
    let rows = db
        .prepare("SELECT data FROM previs_media WHERE project_id=?1 ORDER BY rowid")?
        .query_map([project], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let media = rows
        .into_iter()
        .map(|raw| {
            let mut value: Value = serde_json::from_str(&raw)?;
            value["missing"] = json!(
                !value["path"]
                    .as_str()
                    .is_some_and(|p| std::path::Path::new(p).is_file())
            );
            Ok(value)
        })
        .collect::<Result<Vec<_>>>()?;
    if !media.is_empty() {
        document["previsAssets"] = json!(media);
    }
    Ok(())
}
pub fn find<'a>(document: &'a Value, id: &str) -> Option<&'a Value> {
    ["assets", "previsAssets"]
        .into_iter()
        .find_map(|field| document[field].as_array()?.iter().find(|a| a["id"] == id))
}
pub fn resolved(store: &crate::database::Store, document: &Value) -> Result<Vec<Asset>> {
    let mut assets = store.assets()?;
    assets.retain(|asset| {
        document["assets"]
            .as_array()
            .is_some_and(|items| items.iter().any(|a| a["id"] == asset.id))
    });
    for value in document["previsAssets"].as_array().into_iter().flatten() {
        assets.push(serde_json::from_value(value.clone())?);
    }
    Ok(assets)
}
pub fn managed_path(store: &crate::database::Store, asset: &Asset) -> Result<std::path::PathBuf> {
    use anyhow::ensure;
    let path = std::path::Path::new(&asset.path).canonicalize()?;
    let ordinary = store
        .media_root()
        .join("assets")
        .canonicalize()
        .is_ok_and(|root| path.starts_with(root));
    let private: bool = store.db.lock().unwrap().query_row(
        "SELECT EXISTS(SELECT 1 FROM previs_media WHERE id=?1 AND json_extract(data,'$.path')=?2)",
        params![asset.id, asset.path],
        |r| r.get(0),
    )?;
    ensure!(
        ordinary || private,
        "Media path is outside its registered storage"
    );
    Ok(path)
}
pub fn strip(document: &mut Value) {
    if let Some(fields) = document.as_object_mut() {
        fields.remove("previsAssets");
    }
}
/// A task/history snapshot pins its input; only live shot references follow a rerender.
pub(super) fn retire(
    db: &Connection,
    project: &str,
    candidates: &[String],
    document: &Value,
) -> Result<()> {
    let mut usage = document.clone();
    strip(&mut usage);
    let mut used = HashSet::new();
    crate::project_storage::references(&usage, &mut used);
    for sql in [
        "SELECT data FROM jobs WHERE project_id=?1",
        "SELECT payload FROM agent_messages WHERE project_id=?1",
        "SELECT data FROM previs_exports WHERE project_id=?1",
    ] {
        let rows = db
            .prepare(sql)?
            .query_map([project], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for raw in rows {
            crate::project_storage::references(&serde_json::from_str::<Value>(&raw)?, &mut used);
        }
    }
    for id in candidates.iter().filter(|id| !used.contains(*id)) {
        db.execute(
            "DELETE FROM previs_media WHERE project_id=?1 AND id=?2",
            params![project, id],
        )?;
    }
    Ok(())
}
