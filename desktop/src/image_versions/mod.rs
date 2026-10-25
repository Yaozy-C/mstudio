//! Assets keep their identity; image files and their processing history live here.
mod migration;
#[cfg(test)]
mod tests;
use crate::{database::Store, project_service};
use anyhow::{Context, Result, ensure};
use mstudio::{media, model::Asset};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use tauri::Manager;

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImageVersion {
    pub id: String,
    pub image: Asset,
    pub operation: Option<Value>,
}
pub(super) const SCHEMA: &str = "CREATE TABLE IF NOT EXISTS image_versions(
    project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
    image_id TEXT NOT NULL,id TEXT NOT NULL,data TEXT NOT NULL,provenance TEXT,
    created INTEGER NOT NULL DEFAULT(unixepoch()),PRIMARY KEY(project_id,id));
    CREATE INDEX IF NOT EXISTS image_versions_image ON image_versions(project_id,image_id);";
pub fn init(db: &Connection) -> Result<()> {
    migration::run(db)?;
    db.execute_batch(SCHEMA)?;
    Ok(())
}
pub fn original_id(image: &str) -> String {
    format!("original:{image}")
}
pub fn image_id(asset: &Value) -> Result<&str> {
    asset["imageId"]
        .as_str()
        .or(asset["id"].as_str())
        .context("图片缺少 ID")
}
fn member<'a>(document: &'a Value, id: &str) -> Result<&'a Value> {
    document["assets"]
        .as_array()
        .and_then(|a| a.iter().find(|a| a["id"] == id && a["kind"] == "image"))
        .context("资产不属于当前项目")
}
fn decode(id: String, raw: String, source: Option<String>) -> Result<ImageVersion> {
    let mut image: Asset = serde_json::from_str(&raw)?;
    image.missing = !Path::new(&image.path).is_file();
    let operation = source
        .map(|source| -> Result<Value> {
            let mut operation = serde_json::from_str::<Value>(&source)?["operation"].take();
            if let Some(fields) = operation.as_object_mut() {
                fields.remove("cube");
            }
            Ok(operation)
        })
        .transpose()?;
    Ok(ImageVersion {
        id,
        image,
        operation,
    })
}
/// The query is scoped to an image, independently of asset-library membership.
pub fn list(db: &Connection, project: &str, image: &str) -> Result<Vec<ImageVersion>> {
    let document = project_service::load(db, project)?;
    let asset = document["assets"]
        .as_array()
        .and_then(|a| {
            a.iter()
                .find(|a| a["kind"] == "image" && image_id(a).ok() == Some(image))
        })
        .context("图片不属于当前项目")?;
    let rows=db.prepare("SELECT id,data,provenance FROM image_versions WHERE project_id=?1 AND image_id=?2 ORDER BY provenance IS NOT NULL,created,rowid")?
        .query_map(params![project,image],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,Option<String>>(2)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if rows.is_empty() {
        let mut file = asset.clone();
        file["id"] = json!(image);
        return Ok(vec![decode(original_id(image), file.to_string(), None)?]);
    }
    rows.into_iter()
        .map(|(id, raw, source)| decode(id, raw, source))
        .collect()
}
pub fn input(
    store: &Store,
    project: &str,
    asset_id: &str,
    version: Option<&str>,
) -> Result<(Asset, String)> {
    let db = store.db.lock().unwrap();
    let document = project_service::load(&db, project)?;
    let asset = member(&document, asset_id)?;
    let versions = list(&db, project, image_id(asset)?)?;
    let selected = version.or(asset["imageVersionId"].as_str());
    let input = versions
        .into_iter()
        .find(|v| selected.map_or(v.operation.is_none(), |id| v.id == id))
        .context("版本不属于此图片")?;
    ensure!(!input.image.missing, "图片文件缺失");
    drop(db);
    crate::previs::media::managed_path(store, &input.image)?;
    Ok((input.image, input.id))
}
pub fn save(
    store: &Store,
    project: &str,
    asset_id: &str,
    name: &str,
    outputs: &[PathBuf],
    source: &Value,
) -> Result<Vec<ImageVersion>> {
    let mut imported = Vec::new();
    let result = (|| -> Result<Vec<ImageVersion>> {
        for path in outputs {
            let mut image = media::import(path, &store.media_root())?;
            ensure!(image.kind == "image", "处理输出不是图片");
            image.generated = true;
            image.name = name.into();
            imported.push(image);
        }
        let mut db = store.db.lock().unwrap();
        let tx = db.transaction()?;
        let before = project_service::load(&tx, project)?;
        let asset = member(&before, asset_id)?;
        let image = image_id(asset)?;
        // Keep the original physical file even when the asset later selects a result.
        let mut original = asset.clone();
        original["id"] = json!(image);
        tx.execute(
            "INSERT OR IGNORE INTO image_versions(project_id,image_id,id,data) VALUES(?1,?2,?3,?4)",
            params![project, image, original_id(image), original.to_string()],
        )?;
        for file in &imported {
            tx.execute("INSERT INTO image_versions(project_id,image_id,id,data,provenance) VALUES(?1,?2,?3,?4,?5)",params![project,image,file.id,serde_json::to_string(file)?,source.to_string()])?;
        }
        project_service::persist(&tx, &before, &before)?;
        tx.commit()?;
        Ok(imported
            .iter()
            .map(|image| ImageVersion {
                id: image.id.clone(),
                image: image.clone(),
                operation: Some(source["operation"].clone()),
            })
            .collect())
    })();
    if result.is_err() {
        for image in imported {
            let _ = std::fs::remove_file(image.path);
            if !image.preview.is_empty() {
                let _ = std::fs::remove_file(image.preview);
            }
        }
    }
    result
}
fn apply_file(asset: &mut Value, image: &Value) {
    for key in ["path", "preview", "width", "height", "duration", "hasAudio"] {
        asset[key] = image[key].clone();
    }
    asset["missing"] = json!(
        !image["path"]
            .as_str()
            .is_some_and(|p| Path::new(p).is_file())
    );
}
/// Project assets expose the selected file under the same stable logical ID.
/// The document retains its version selection, including saved input snapshots.
pub fn hydrate(db: &Connection, project: &str, document: &mut Value) -> Result<()> {
    for asset in document["assets"].as_array_mut().into_iter().flatten() {
        if asset["kind"] != "image" {
            continue;
        }
        let image = image_id(asset)?.to_owned();
        let selected = asset["imageVersionId"]
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| original_id(&image));
        let raw: Option<String> = db
            .query_row(
                "SELECT data FROM image_versions WHERE project_id=?1 AND image_id=?2 AND id=?3",
                params![project, image, selected],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(raw) = raw {
            apply_file(asset, &serde_json::from_str::<Value>(&raw)?);
        }
        asset["imageId"] = json!(image);
        asset["imageVersionId"] = json!(selected);
        let count: u64 = db.query_row(
            "SELECT count(*) FROM image_versions WHERE project_id=?1 AND image_id=?2",
            params![project, image],
            |r| r.get(0),
        )?;
        asset["imageVersionCount"] = json!(count.max(1));
        let fields = asset.as_object_mut().context("图片元数据无效")?;
        for key in ["legacyImageVersion", "discarded"] {
            fields.remove(key);
        }
    }
    Ok(())
}
/// Selecting a picture version updates only this asset's choice, never its ID.
pub fn select(store: &Store, project: &str, asset_id: &str, version: &str) -> Result<Value> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let before = project_service::load(&tx, project)?;
    let asset = member(&before, asset_id)?;
    let chosen = list(&tx, project, image_id(asset)?)?
        .into_iter()
        .find(|v| v.id == version)
        .context("版本不属于此图片")?;
    ensure!(!chosen.image.missing, "图片文件缺失，无法设为参考");
    let mut after = before.clone();
    let asset = after["assets"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|a| a["id"] == asset_id)
        .unwrap();
    apply_file(asset, &json!(chosen.image));
    asset["imageVersionId"] = json!(version);
    asset["inLibrary"] = json!(true);
    let selected = asset.clone();
    project_service::persist(&tx, &before, &after)?;
    tx.commit()?;
    store.project_changed(project);
    Ok(selected)
}
#[tauri::command]
pub fn list_image_versions(
    store: tauri::State<Store>,
    project_id: String,
    image_id: String,
) -> Result<Vec<ImageVersion>, String> {
    list(&store.db.lock().unwrap(), &project_id, &image_id).map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn select_image_reference(
    app: tauri::AppHandle,
    project_id: String,
    asset_id: String,
    version_id: String,
) -> Result<Value, String> {
    let store = app.state::<Store>();
    let _guard = crate::project_storage::working(&store, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    let asset = select(&store, &project_id, &asset_id, &version_id).map_err(|e| e.to_string())?;
    project_service::notify(&app, &project_id);
    Ok(asset)
}
