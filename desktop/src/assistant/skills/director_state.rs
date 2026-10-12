//! Upgrade shipped direction rules without overwriting authored skill text.
use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub(super) fn migrate(db: &Connection, root: &Path) -> Result<()> {
    let manifest = serde_json::from_str(include_str!("director_physical_state_migration.json"))?;
    super::rule_summary::apply(db, root, manifest, "director_physical_state_v1")
}
