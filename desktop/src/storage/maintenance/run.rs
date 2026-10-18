use super::{Report, scan};
use crate::database::Store;
use anyhow::{Result, ensure};
use rusqlite::params;
use serde::Serialize;
use std::{collections::HashSet, path::Path};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub report: Report,
    pub freed_bytes: u64,
    pub deleted_files: u64,
    pub deleted_records: u64,
    pub warning: String,
    #[serde(skip)]
    pub changed_projects: Vec<String>,
}
pub fn clean(store: &Store, data: &Path, fingerprint: &str) -> Result<Outcome> {
    let root = store.media_root();
    let mut db = store.conn()?;
    let plan = scan::build(&db, &root, data)?;
    ensure!(
        plan.report.can_clean,
        "有生成或 Agent 任务尚未结束，请等待结束后再清理"
    );
    ensure!(
        plan.report.fingerprint == fingerprint,
        "文件或引用已变化，请重新扫描后再清理"
    );
    let before_bytes = plan.report.total_bytes;
    let dead: HashSet<String> = plan
        .graph
        .assets
        .iter()
        .filter_map(|a| a["id"].as_str())
        .filter(|id| !plan.graph.ids.contains(*id))
        .map(str::to_owned)
        .collect();
    let tx = db.transaction()?;
    let mut changed_projects = vec![];
    for (id, before) in &plan.graph.documents {
        let mut after = before.clone();
        if let Some(assets) = after["assets"].as_array_mut() {
            assets.retain(|a| {
                a["id"]
                    .as_str()
                    .is_none_or(|id| plan.graph.ids.contains(id))
            });
        }
        if after != *before {
            crate::project_service::persist(&tx, before, &after)?;
            changed_projects.push(id.clone());
        }
    }
    let mut deleted_records = 0;
    for id in dead {
        tx.execute("DELETE FROM project_assets WHERE asset_id=?1", [&id])?;
        deleted_records += tx.execute("DELETE FROM assets WHERE id=?1", [&id])? as u64;
    }
    for (project, asset) in &plan.graph.private {
        let id = asset["id"].as_str().unwrap();
        if !plan.graph.ids.contains(id) {
            deleted_records += tx.execute(
                "DELETE FROM previs_media WHERE project_id=?1 AND id=?2",
                params![project, id],
            )? as u64;
            if !changed_projects.contains(project) {
                changed_projects.push(project.clone());
            }
        }
    }
    deleted_records += tx.execute("DELETE FROM content_blobs WHERE NOT EXISTS(SELECT 1 FROM event_content WHERE digest=content_blobs.digest) AND NOT EXISTS(SELECT 1 FROM job_content WHERE digest=content_blobs.digest)", [])? as u64;
    deleted_records += tx.execute("DELETE FROM stored_media WHERE NOT EXISTS(SELECT 1 FROM content_blobs WHERE media_digest=stored_media.digest)", [])? as u64;
    // Queue only individually scanned files, never a directory with possible shared users.
    for path in &plan.files {
        tx.execute(
            "INSERT OR IGNORE INTO pending_file_deletions VALUES('storage-cleanup',?1)",
            [path.to_string_lossy().as_ref()],
        )?;
        tx.execute(
            "DELETE FROM project_files WHERE path=?1",
            [path.to_string_lossy().as_ref()],
        )?;
    }
    let ownership = tx.prepare("SELECT DISTINCT asset_id FROM project_assets WHERE NOT EXISTS(SELECT 1 FROM assets WHERE id=asset_id)")?
        .query_map([], |r| r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    for id in ownership
        .into_iter()
        .filter(|id| !plan.graph.ids.contains(id))
    {
        tx.execute("DELETE FROM project_assets WHERE asset_id=?1", [&id])?;
        deleted_records += 1;
    }
    tx.commit()?;
    let mut warnings = vec![];
    if let Err(error) = crate::project_storage::resume_cleanup(&db, &root) {
        warnings.push(error.to_string());
    }
    // Reclaim SQLite's freed pages as well as physical media files.
    if let Err(error) = db
        .execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM; PRAGMA wal_checkpoint(TRUNCATE);")
    {
        warnings.push(format!("记录已清理，数据库空间整理未完成：{error}"));
    }
    remove_empty(&root);
    let report = scan::build(&db, &root, data)?.report;
    let deleted_files = plan.files.iter().filter(|p| !p.exists()).count() as u64;
    Ok(Outcome {
        freed_bytes: before_bytes.saturating_sub(report.total_bytes),
        report,
        deleted_files,
        deleted_records,
        warning: warnings.join("；"),
        changed_projects,
    })
}

fn remove_empty(root: &Path) {
    fn prune(root: &Path, path: &Path) {
        if crate::project_storage::managed(root, path)
            .ok()
            .flatten()
            .is_none()
        {
            return;
        }
        let Ok(meta) = std::fs::symlink_metadata(path) else {
            return;
        };
        if meta.file_type().is_symlink() || !meta.is_dir() {
            return;
        }
        if let Ok(entries) = std::fs::read_dir(path) {
            for entry in entries.flatten() {
                prune(root, &entry.path());
            }
        }
        let _ = std::fs::remove_dir(path);
    }
    for folder in crate::storage::FOLDERS {
        if std::fs::symlink_metadata(root.join(folder)).is_ok_and(|m| m.file_type().is_symlink()) {
            continue;
        }
        if let Ok(entries) = std::fs::read_dir(root.join(folder)) {
            for entry in entries.flatten() {
                prune(root, &entry.path());
            }
        }
    }
}
