//! Update shipped concept handoff rules while preserving user-authored text.
use anyhow::Result;
use rusqlite::Connection;
use std::path::Path;

pub(super) fn migrate(db: &Connection, root: &Path) -> Result<()> {
    let manifest = serde_json::from_str(include_str!("concept_handoff_migration.json"))?;
    super::rule_summary::apply(db, root, manifest, "concept_handoff_v1")
}
