//! Current delete requests record IDs in the same transaction as their project.
use anyhow::Result;
use rusqlite::{Connection, params};
use std::collections::HashSet;

pub(super) fn init(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS project_removed_assets(
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        asset_id TEXT NOT NULL, PRIMARY KEY(project_id,asset_id));",
    )?;
    Ok(())
}
pub(super) fn record(db: &Connection, project: &str, removed: &HashSet<String>) -> Result<()> {
    let mut insert =
        db.prepare_cached("INSERT OR IGNORE INTO project_removed_assets VALUES(?1,?2)")?;
    for id in removed {
        insert.execute(params![project, id])?;
    }
    Ok(())
}
pub(super) fn read(db: &Connection, project: &str) -> Result<HashSet<String>> {
    Ok(db
        .prepare("SELECT asset_id FROM project_removed_assets WHERE project_id=?1")?
        .query_map([project], |r| r.get(0))?
        .collect::<rusqlite::Result<_>>()?)
}

#[cfg(test)]
#[path = "deletion_tombstone_tests.rs"]
mod tests;
