//! Patch known shipped contract errors without replacing user-owned Skill bodies.
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use std::collections::BTreeMap;

const MARKER: &str = "skill_contracts_v1";
#[derive(Deserialize)]
struct Edit {
    skill: String,
    path: String,
    before: String,
    after: String,
}
fn edits() -> Vec<Edit> {
    serde_json::from_str(include_str!("contract_migration.json"))
        .expect("valid Skill contract corrections")
}
pub fn migrate(db: &Connection) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    if tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key=?1)",
        [MARKER],
        |r| r.get::<_, bool>(0),
    )? {
        return Ok(());
    }
    let mut resources: BTreeMap<(String, String), Vec<Edit>> = BTreeMap::new();
    for edit in edits() {
        resources
            .entry((edit.skill.clone(), edit.path.clone()))
            .or_default()
            .push(edit);
    }
    for ((skill, path), edits) in resources {
        let original: Option<String> = tx
            .query_row(
                "SELECT text FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![skill, path],
                |r| r.get(0),
            )
            .optional()?;
        let Some(original) = original else { continue };
        let mut updated = original.clone();
        for edit in edits {
            updated = updated.replace(&edit.before, &edit.after);
        }
        if updated != original {
            tx.execute("UPDATE skill_resources SET text=?3,revision=revision+1,updated=unixepoch() WHERE skill_id=?1 AND path=?2", params![skill,path,updated])?;
        }
    }
    tx.execute(
        "INSERT INTO settings(key,value) VALUES(?1,'true')",
        [MARKER],
    )?;
    tx.commit()?;
    Ok(())
}
#[cfg(test)]
#[path = "contract_migration_tests.rs"]
mod tests;
