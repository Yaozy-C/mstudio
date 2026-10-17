use super::{files, render::Export};
use crate::{database::Store, project_service, project_storage};
use anyhow::{Context, Result, ensure};
use mstudio::media;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::{collections::HashSet, path::PathBuf, sync::Mutex};

// Serialize publication and reference selection; rendering itself remains concurrent.
pub(super) static WRITE: Mutex<()> = Mutex::new(());
fn list(db: &Connection, project: &str, file: Option<&str>) -> Result<Vec<Export>> {
    db.prepare("SELECT data FROM previs_exports WHERE project_id=?1 AND (?2 IS NULL OR json_extract(data,'$.fileId')=?2) ORDER BY rowid DESC")?
        .query_map(params![project, file], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?.iter().map(|raw| serde_json::from_str(raw).map_err(Into::into)).collect()
}

pub(super) fn replace(store: &Store, project: &str, export: Export) -> Result<Export> {
    let _guard = WRITE.lock().unwrap();
    let old = list(&store.db.lock().unwrap(), project, Some(&export.file_id))?;
    commit(store, project, export, old, true)
}
/// Coalesce existing stacks on workspace load, including references to an older export.
pub(super) fn normalize(store: &Store, project: &str) -> Result<bool> {
    let _guard = WRITE.lock().unwrap();
    let all = list(&store.db.lock().unwrap(), project, None)?;
    let mut seen = HashSet::new();
    let mut changed = false;
    for export in &all {
        if !seen.insert(export.file_id.clone()) {
            continue;
        }
        let old: Vec<_> = all
            .iter()
            .filter(|old| old.file_id == export.file_id)
            .cloned()
            .collect();
        if old.len() > 1 {
            commit(store, project, export.clone(), old, false)?;
            changed = true;
        }
    }
    Ok(changed)
}
fn commit(
    store: &Store,
    project: &str,
    mut export: Export,
    old: Vec<Export>,
    fresh: bool,
) -> Result<Export> {
    let document = project_service::load(&store.db.lock().unwrap(), project)?;
    let old_ids: HashSet<_> = old
        .iter()
        .filter_map(|video| video.shared_asset_id.as_deref())
        .collect();
    let mut references = HashSet::new();
    project_storage::references(&document["nodes"], &mut references);
    let shared = document["assets"]
        .as_array()
        .context("素材目录无效")?
        .iter()
        .any(|asset| {
            asset["id"].as_str().is_some_and(|id| old_ids.contains(id))
                && asset["discarded"] != true
                && asset["missing"] != true
        })
        || references.iter().any(|id| old_ids.contains(id.as_str()));
    let mut prepared: Option<PathBuf> = None;
    let mut copy: Option<Value> = None;
    if shared {
        copy = if !fresh {
            export.shared_asset_id.as_ref().and_then(|id| {
                document["assets"]
                    .as_array()?
                    .iter()
                    .find(|asset| {
                        asset["id"] == *id && asset["missing"] != true && asset["discarded"] != true
                    })
                    .cloned()
            })
        } else {
            None
        };
        if copy.is_none() {
            let directory = store
                .media_root()
                .join("exports")
                .join(format!("previs-copy-{}", media::id()));
            project_storage::track_file(store, project, &directory)?;
            let imported = media::import(std::path::Path::new(&export.asset.path), &directory);
            let mut asset = match imported {
                Ok(asset) => asset,
                Err(error) => {
                    let _ = crate::imports::discard(store, project, &directory);
                    return Err(error);
                }
            };
            asset.name = format!("{}.mp4", export.asset.name.trim_end_matches(".mp4"));
            let mut value = serde_json::to_value(&asset)?;
            value["inLibrary"] = json!(true);
            copy = Some(value);
            prepared = Some(directory);
        }
    }
    let result = (|| -> Result<Export> {
        let mut db = store.db.lock().unwrap();
        let tx = db.transaction()?;
        let before = project_service::load(&tx, project)?;
        if fresh {
            let current = files::read(&tx, project, &export.file_id)?.context("镜头已删除")?;
            ensure!(
                current.revision == export.file_revision,
                "镜头已修改，本次渲染未替换现有预演"
            );
            for (id, revision) in &export.model_revisions {
                ensure!(
                    files::read(&tx, project, id)?.is_some_and(|model| model.revision == *revision),
                    "模型已修改，本次渲染未替换现有预演"
                );
            }
        }
        let mut after = before.clone();
        if let Some(asset) = &copy {
            export.shared_asset_id =
                Some(asset["id"].as_str().context("预演素材 ID 缺失")?.to_owned());
            tx.execute(
                "INSERT INTO assets VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
                params![asset["id"].as_str(), asset.to_string()],
            )?;
            super::export_references::update(&mut after, &old, asset)?;
        } else {
            export.shared_asset_id = None;
        }
        // Keep one stable result identity while replacing its bytes and source signature.
        if let Some(previous) = old.first() {
            export.id = previous.id.clone();
        }
        tx.execute(
            "DELETE FROM previs_exports WHERE project_id=?1 AND json_extract(data,'$.fileId')=?2",
            params![project, export.file_id],
        )?;
        tx.execute(
            "INSERT INTO previs_exports VALUES(?1,?2,?3)",
            params![project, export.id, serde_json::to_string(&export)?],
        )?;
        if before != after {
            project_service::persist(&tx, &before, &after)?;
        }
        super::export_cleanup::retire(&tx, store, project, &old, &export)?;
        tx.commit()?;
        // Publication is complete even if a locked file needs deferred cleanup.
        let _ = project_storage::resume_cleanup(&db, &store.media_root());
        drop(db);
        if before != after {
            store.project_changed(project);
        }
        Ok(export.clone())
    })();
    if result.is_err()
        && let Some(directory) = prepared
    {
        let _ = crate::imports::discard(store, project, &directory);
    }
    result
}
