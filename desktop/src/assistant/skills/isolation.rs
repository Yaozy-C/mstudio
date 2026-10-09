//! Localize shipped rule dependencies without overwriting authored documents.
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::path::Path;

const MARKER: &str = "skill_isolation_v1";
#[derive(Deserialize)]
struct Resource {
    skill: String,
    path: String,
    before: Option<String>,
}
pub(super) fn migrate(db: &Connection, root: &Path) -> Result<()> {
    if db.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key=?1)",
        [MARKER],
        |r| r.get::<_, bool>(0),
    )? {
        return Ok(());
    }
    let entries: Vec<Resource> = serde_json::from_str(include_str!("isolation_migration.json"))?;
    let tx = db.unchecked_transaction()?;
    for entry in entries {
        let body = std::fs::read_to_string(root.join(&entry.skill).join(&entry.path))?;
        let old: Option<String> = tx
            .query_row(
                "SELECT text FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![entry.skill, entry.path],
                |r| r.get(0),
            )
            .optional()?;
        match old {
            None if entry.before.is_none() => {
                tx.execute(
                    "INSERT INTO skill_resources(skill_id,path,text) VALUES(?1,?2,?3)",
                    params![entry.skill, entry.path, body],
                )?;
            }
            Some(old)
                if entry.before.as_deref()
                    == Some(&format!("{:x}", Sha256::digest(old.as_bytes())))
                    && old != body =>
            {
                tx.execute("UPDATE skill_resources SET text=?3,revision=revision+1,updated=unixepoch() WHERE skill_id=?1 AND path=?2", params![entry.skill, entry.path, body])?;
            }
            _ => {} // Custom bodies, including existing local methods, remain authoritative.
        }
    }
    let agents: Option<String> = tx
        .query_row("SELECT value FROM settings WHERE key='agents'", [], |r| {
            r.get(0)
        })
        .optional()?;
    if let Some(agents) = agents {
        let mut profiles: Vec<serde_json::Value> = serde_json::from_str(&agents)?;
        for profile in &mut profiles {
            let old = profile["instructions"].as_str().unwrap_or_default();
            let new = match profile["id"].as_str() {
                Some("editor") => old.replace(
                    "creative-ad-director/references/rhythm.md",
                    "video-editing/references/rhythm.md",
                ),
                Some("image") => old.replace(
                    "use shared camera or material methods only for this still",
                    "use its local camera and material methods only for this still",
                ),
                Some("production") => old.replace(
                    "Read product-video-production and relevant shared methods.",
                    "Read product-video-production and its relevant local execution methods.",
                ),
                _ => old.to_owned(),
            };
            if new != old {
                profile["revision"] = (profile["revision"].as_u64().unwrap_or(0) + 1).into();
                profile["instructions"] = new.into();
            }
        }
        tx.execute(
            "UPDATE settings SET value=?1 WHERE key='agents'",
            [serde_json::to_string(&profiles)?],
        )?;
    }
    tx.execute(
        "INSERT INTO settings(key,value) VALUES(?1,'true')",
        [MARKER],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn installs_local_methods_preserves_custom_documents_and_editor_text() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            .unwrap();
        super::super::storage::init(&db).unwrap();
        db.execute("INSERT INTO skill_resources(skill_id,path,text) VALUES('image-production','SKILL.md','CUSTOM ENTRY'),('image-production','references/scene-execution.md','CUSTOM LOCAL METHOD')", []).unwrap();
        let agents = serde_json::json!([{"id":"editor","revision":12,"instructions":"CUSTOM creative-ad-director/references/rhythm.md SUFFIX","enabled":false,"skillIds":["video-editing"]}]);
        db.execute(
            "INSERT INTO settings VALUES('agents',?1)",
            [agents.to_string()],
        )
        .unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills");
        migrate(&db, &root).unwrap();
        for (path, expected) in [
            ("SKILL.md", "CUSTOM ENTRY"),
            ("references/scene-execution.md", "CUSTOM LOCAL METHOD"),
        ] {
            let text: String = db.query_row("SELECT text FROM skill_resources WHERE skill_id='image-production' AND path=?1", [path], |r| r.get(0)).unwrap();
            assert_eq!(text, expected);
        }
        let raw: String = db
            .query_row("SELECT value FROM settings WHERE key='agents'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let updated: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(
            updated[0]["instructions"],
            "CUSTOM video-editing/references/rhythm.md SUFFIX"
        );
        assert_eq!(updated[0]["revision"], 13);
        assert_eq!(updated[0]["enabled"], false);
        let count: i64 = db
            .query_row("SELECT count(*) FROM skill_resources", [], |r| r.get(0))
            .unwrap();
        migrate(&db, &root).unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM skill_resources", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            count
        );
        assert_eq!(
            db.query_row("SELECT value FROM settings WHERE key='agents'", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
            raw
        );
    }
}
