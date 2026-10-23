use super::render::Export;
use crate::database::Store;
use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

fn directory(store: &Store, video: &Export) -> Option<PathBuf> {
    let root = store.media_root().join("exports");
    let path = Path::new(&video.asset.path).strip_prefix(&root).ok()?;
    let folder = path.components().next()?.as_os_str().to_str()?;
    if !folder.starts_with("previs-") || folder.contains(['/', '\\']) {
        return None;
    }
    Some(root.join(folder))
}
fn protected(db: &Connection, prefix: &str) -> Result<bool> {
    for (table, column) in [
        ("assets", "data"),
        ("jobs", "data"),
        ("agent_messages", "payload"),
        ("previs_exports", "data"),
        ("previs_media", "data"),
    ] {
        // Inspect decoded JSON strings: Windows separators and quotes are escaped
        // in the stored document. Match a directory boundary, not a sibling prefix.
        let query = format!(
            "SELECT EXISTS(SELECT 1 FROM {table}, json_tree({column}) AS entry
             WHERE entry.type='text' AND (entry.atom=?1 OR
             substr(entry.atom,1,length(?1)+1) IN (?1||'/',?1||char(92))))"
        );
        if db.query_row(&query, [prefix], |r| r.get::<_, bool>(0))? {
            return Ok(true);
        }
    }
    Ok(false)
}
pub(super) fn retire(
    db: &Connection,
    store: &Store,
    project: &str,
    old: &[Export],
    current: &Export,
) -> Result<()> {
    let keep = directory(store, current);
    for video in old {
        let Some(path) = directory(store, video) else {
            continue;
        };
        if Some(&path) == keep.as_ref() {
            continue;
        }
        let prefix = path.to_string_lossy();
        // Private renders normally have no external users. Preserve any exceptional snapshot.
        if protected(db, &prefix)? {
            continue;
        }
        db.execute(
            "DELETE FROM project_files WHERE project_id=?1 AND path=?2",
            params![project, prefix],
        )?;
        db.execute(
            "INSERT OR IGNORE INTO pending_file_deletions VALUES(?1,?2)",
            params![project, prefix],
        )?;
    }
    Ok(())
}

#[cfg(test)]
#[path = "export_cleanup_tests.rs"]
mod tests;
