use super::{files, render::Export};
use crate::{database::Store, project_service, project_storage};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, params};
use std::sync::Mutex;

pub(super) static WRITE: Mutex<()> = Mutex::new(());
fn list(db: &Connection, project: &str, file: Option<&str>) -> Result<Vec<Export>> {
    db.prepare("SELECT data FROM previs_exports WHERE project_id=?1 AND (?2 IS NULL OR json_extract(data,'$.fileId')=?2) ORDER BY rowid DESC")?
        .query_map(params![project, file], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?.iter().map(|raw| serde_json::from_str(raw).map_err(Into::into)).collect()
}
pub(super) fn replace(store: &Store, project: &str, mut export: Export) -> Result<Export> {
    let _guard = WRITE.lock().unwrap();
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let old = list(&tx, project, Some(&export.file_id))?;
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
    ensure!(
        std::path::Path::new(&export.asset.path).is_file(),
        "预演视频文件不存在"
    );
    if let Some(previous) = old.first() {
        export.id = previous.id.clone();
    }
    export.asset.generated = true;
    super::media::register(&tx, project, &export.asset)?;
    let before = project_service::load(&tx, project)?;
    let mut after = before.clone();
    super::export_references::update(&mut after, &old, &serde_json::to_value(&export.asset)?)?;
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
    let retired: Vec<String> = old.iter().map(|old| old.asset.id.clone()).collect();
    super::media::retire(&tx, project, &retired, &after)?;
    super::export_cleanup::retire(&tx, store, project, &old, &export)?;
    tx.commit()?;
    let _ = project_storage::resume_cleanup(&db, &store.media_root());
    drop(db);
    store.project_changed(project);
    Ok(export)
}

pub fn read_export(db: &rusqlite::Connection, project: &str, id: &str) -> Result<Export> {
    let raw: String = db
        .query_row(
            "SELECT data FROM previs_exports WHERE project_id=?1 AND id=?2",
            params![project, id],
            |r| r.get(0),
        )
        .context("预演视频不存在")?;
    Ok(serde_json::from_str(&raw)?)
}
