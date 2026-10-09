//! Install the separated concept method without overwriting authored rules.
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use sha2::{Digest, Sha256};
use std::path::Path;

pub(super) fn migrate(db: &Connection, root: &Path) -> Result<()> {
    const MARKER: &str = "creative_screenwriting_split_v1";
    let done: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key=?1)",
        [MARKER],
        |r| r.get(0),
    )?;
    if done {
        return Ok(());
    }
    let concept = std::fs::read_to_string(root.join("creative-concepts/SKILL.md"))?;
    let tx = db.unchecked_transaction()?;
    tx.execute("INSERT OR IGNORE INTO skill_resources(skill_id,path,text) VALUES('creative-concepts','SKILL.md',?1)", [concept])?;
    for (path, digest) in [
        (
            "SKILL.md",
            "71cc2db83b4b2a557cae188c1325755d178f5814c9dbe865e661a5e70d44fa3e",
        ),
        (
            "CORE.md",
            "3db946e9755dfc1c4b76b7c320c97660a6d9dd143cecab04f195fa4b8549cd45",
        ),
        (
            "references/craft.md",
            "af6f9b22558c21d677d0354951c477765701fae2620f0034e7fb420127cc40fd",
        ),
    ] {
        let body = std::fs::read_to_string(root.join("ad-script").join(path))?;
        replace_default(&tx, "ad-script", path, digest, &body)?;
    }
    let direction = std::fs::read_to_string(root.join("creative-ad-director/SKILL.md"))?;
    replace_default(
        &tx,
        "creative-ad-director",
        "SKILL.md",
        "fc0013ecf8645342493bc318651a5c78d22ccb9e12d0fc22c48a2c088d0da64f",
        &direction,
    )?;
    tx.execute(
        "INSERT INTO settings(key,value) VALUES(?1,'true')",
        [MARKER],
    )?;
    tx.commit()?;
    Ok(())
}

fn replace_default(
    db: &Connection,
    owner: &str,
    path: &str,
    expected: &str,
    replacement: &str,
) -> Result<()> {
    let old: Option<String> = db
        .query_row(
            "SELECT text FROM skill_resources WHERE skill_id=?1 AND path=?2",
            params![owner, path],
            |r| r.get(0),
        )
        .optional()?;
    if let Some(old) = old
        && format!("{:x}", Sha256::digest(old.as_bytes())) == expected
        && old != replacement
    {
        db.execute("UPDATE skill_resources SET text=?3,revision=revision+1,updated=unixepoch() WHERE skill_id=?1 AND path=?2", params![owner, path, replacement])?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn split_installs_missing_package_and_preserves_custom_rules_and_is_idempotent() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            .unwrap();
        crate::assistant::skills::storage::init(&db).unwrap();
        db.execute("INSERT INTO skill_resources(skill_id,path,text) VALUES('ad-script','SKILL.md','CUSTOM WRITING RULE')", []).unwrap();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills");
        migrate(&db, &root).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT text FROM skill_resources WHERE skill_id='ad-script'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "CUSTOM WRITING RULE"
        );
        let before: String = db
            .query_row(
                "SELECT text FROM skill_resources WHERE skill_id='creative-concepts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(
            before,
            std::fs::read_to_string(root.join("creative-concepts/SKILL.md")).unwrap()
        );
        db.execute("UPDATE skill_resources SET text='CUSTOM CONCEPT RULE' WHERE skill_id='creative-concepts'", []).unwrap();
        migrate(&db, &root).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT text FROM skill_resources WHERE skill_id='creative-concepts'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "CUSTOM CONCEPT RULE"
        );
    }
    #[test]
    fn recognized_default_is_replaced_once_while_other_bodies_survive() {
        let db = Connection::open_in_memory().unwrap();
        crate::assistant::skills::storage::init(&db).unwrap();
        db.execute("INSERT INTO skill_resources(skill_id,path,text) VALUES('ad-script','CORE.md','OLD DEFAULT')", []).unwrap();
        let digest = format!("{:x}", Sha256::digest(b"OLD DEFAULT"));
        replace_default(&db, "ad-script", "CORE.md", &digest, "SEPARATED WRITING").unwrap();
        replace_default(&db, "ad-script", "CORE.md", &digest, "MUST NOT OVERWRITE").unwrap();
        let (body, revision): (String, i64) = db
            .query_row(
                "SELECT text,revision FROM skill_resources WHERE skill_id='ad-script'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(body, "SEPARATED WRITING");
        assert_eq!(revision, 2);
    }
}
