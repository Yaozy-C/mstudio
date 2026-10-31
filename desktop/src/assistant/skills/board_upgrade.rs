//! Upgrade recognized shipped concept rules while preserving authored database text.
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn apply(db: &Connection, root: &Path) -> Result<()> {
    for (path, previous) in [
        (
            "SKILL.md",
            "bf1d31f522c59343d85c40f933d0d8710c2071394d26e1f6ac6c49884abf61c8",
        ),
        (
            "references/diagnosis.md",
            "96509f18ac9f7cf512923965156c07e69c92913aeba5b239b074ab418fe1c7ec",
        ),
        (
            "references/discovery-and-development.md",
            "ce558d881ab3dc7755e6e678ce5f09806d10f3a75bb6624c2e187d13cb27cb43",
        ),
    ] {
        let next = std::fs::read_to_string(root.join("creative-concepts").join(path))?;
        replace_known(db, path, previous, &next)?;
    }
    Ok(())
}

fn replace_known(db: &Connection, path: &str, previous: &str, next: &str) -> Result<()> {
    let text: Option<String> = db
        .query_row(
            "SELECT text FROM skill_resources WHERE skill_id='creative-concepts' AND path=?1",
            [path],
            |row| row.get(0),
        )
        .optional()?;
    if let Some(text) = text
        && text != next
        && format!("{:x}", Sha256::digest(text.as_bytes())) == previous
    {
        // Keep an exact source copy as well as the normal event history for recovery.
        db.execute(
            "INSERT OR IGNORE INTO settings(key,value) VALUES(?1,?2)",
            params![format!("concept_board_rules_v2:{path}"), text],
        )?;
        db.execute("UPDATE skill_resources SET text=?2,revision=revision+1,updated=unixepoch() WHERE skill_id='creative-concepts' AND path=?1", params![path, next])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn upgrades_known_rules_once_and_preserves_custom_edits_and_backup() {
        let db = Connection::open_in_memory().unwrap();
        super::super::storage::init(&db).unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);")
            .unwrap();
        db.execute("INSERT INTO skill_resources(skill_id,path,text) VALUES('creative-concepts','SKILL.md','old shipped method')", []).unwrap();
        let hash = format!("{:x}", Sha256::digest(b"old shipped method"));
        replace_known(&db, "SKILL.md", &hash, "new coherent method").unwrap();
        replace_known(&db, "SKILL.md", &hash, "new coherent method").unwrap();
        let saved: (String, i64) = db
            .query_row("SELECT text,revision FROM skill_resources", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(saved, ("new coherent method".into(), 2));
        let backup: String = db
            .query_row(
                "SELECT value FROM settings WHERE key='concept_board_rules_v2:SKILL.md'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(backup, "old shipped method");
        db.execute("UPDATE skill_resources SET text='my custom method'", [])
            .unwrap();
        replace_known(&db, "SKILL.md", &hash, "new coherent method").unwrap();
        let custom: (String, i64) = db
            .query_row("SELECT text,revision FROM skill_resources", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert_eq!(custom, ("my custom method".into(), 2));
    }
}
