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
        let protected = [
            "SELECT data FROM assets",
            "SELECT data FROM jobs",
            "SELECT payload FROM agent_messages",
            "SELECT data FROM previs_exports",
            "SELECT data FROM previs_media",
        ]
        .iter()
        .any(|sql| {
            let query = format!(
                "SELECT EXISTS(SELECT 1 FROM ({sql}) WHERE instr({},?1)>0)",
                if sql.contains("payload") {
                    "payload"
                } else {
                    "data"
                }
            );
            db.query_row(&query, [prefix.as_ref()], |r| r.get::<_, bool>(0))
                .unwrap_or(true)
        });
        if protected {
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
