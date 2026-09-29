//! Update targeted rule passages without replacing unrelated user edits.
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
#[derive(Deserialize)]
struct Edit {
    skill: String,
    path: String,
    before: String,
    after: String,
}
pub fn migrate(db: &Connection) -> Result<()> {
    if db.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key='prompt_rules_v2')",
        [],
        |r| r.get::<_, bool>(0),
    )? {
        return Ok(());
    }
    crate::model_adapters::prompt_rules::init(db)?;
    let backup = if let Some(path) = db.path().filter(|p| !p.is_empty()) {
        let dest = format!(
            "{path}.before-prompt-rules-{}.sqlite3",
            mstudio::media::id()
        );
        db.execute("VACUUM INTO ?1", [&dest])?;
        dest
    } else {
        String::new()
    };
    let tx = db.unchecked_transaction()?;
    let h3: Option<String> = tx.query_row("SELECT text FROM skill_resources WHERE skill_id='creative-ad-director' AND path='references/h3-prompts.md'", [], |r|r.get(0)).optional()?;
    let h3 = h3.map(|s| s.replace("Read only when H3 is selected. First apply [general video conversion](video-prompt-writing.md).", "Injected only for the selected MiniMax H3 model. Apply the separately loaded general creative rules first."));
    tx.execute(
        "INSERT OR IGNORE INTO model_prompt_rules(id,text) VALUES('minimax-h3',?1)",
        [h3.as_deref()
            .unwrap_or(crate::model_adapters::prompt_rules::H3)],
    )?;
    let edits: Vec<Edit> = serde_json::from_str(include_str!("prompt_rule_migration.json"))?;
    let current_seed: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM skill_resources WHERE skill_id='image-prompt')",
        [],
        |r| r.get(0),
    )?;
    for edit in edits.into_iter().filter(|_| !current_seed) {
        let old: Option<String> = tx
            .query_row(
                "SELECT text FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![edit.skill, edit.path],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(old) = old {
            if old.contains(&edit.after) {
                continue;
            }
            let new = old.replace(&edit.before, &edit.after);
            if new != old {
                tx.execute("UPDATE skill_resources SET text=?3,revision=revision+1,updated=unixepoch() WHERE skill_id=?1 AND path=?2", params![edit.skill,edit.path,new])?;
            }
        }
    }
    tx.execute("DELETE FROM skill_resources WHERE skill_id='creative-ad-director' AND path='references/h3-prompts.md'", [])?;
    tx.execute(
        "INSERT INTO settings(key,value) VALUES('prompt_rules_v2',?1)",
        [backup],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn fresh_seed_does_not_duplicate_required_rules() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            .unwrap();
        super::super::storage::seed(
            &db,
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills"),
        )
        .unwrap();
        let before: String = db.query_row("SELECT text FROM skill_resources WHERE skill_id='creative-ad-director' AND path='SKILL.md'", [], |r|r.get(0)).unwrap();
        migrate(&db).unwrap();
        let after: String = db.query_row("SELECT text FROM skill_resources WHERE skill_id='creative-ad-director' AND path='SKILL.md'", [], |r|r.get(0)).unwrap();
        assert_eq!(before, after);
    }
    #[test]
    fn migration_preserves_custom_rules_moves_model_text_and_runs_once() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            .unwrap();
        super::super::storage::init(&db).unwrap();
        let edits: Vec<Edit> =
            serde_json::from_str(include_str!("prompt_rule_migration.json")).unwrap();
        let edit = &edits[0];
        db.execute(
            "INSERT INTO skill_resources(skill_id,path,text) VALUES(?1,?2,?3)",
            params![
                edit.skill,
                edit.path,
                format!("{}\nCUSTOM_USER_RULE", edit.before)
            ],
        )
        .unwrap();
        db.execute("INSERT INTO skill_resources(skill_id,path,text) VALUES('creative-ad-director','references/h3-prompts.md','CUSTOM_MODEL_RULE')", []).unwrap();
        migrate(&db).unwrap();
        let text: String = db
            .query_row(
                "SELECT text FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![edit.skill, edit.path],
                |r| r.get(0),
            )
            .unwrap();
        assert!(text.contains(&edit.after));
        assert!(text.contains("CUSTOM_USER_RULE"));
        assert_eq!(
            db.query_row(
                "SELECT text FROM model_prompt_rules WHERE id='minimax-h3'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "CUSTOM_MODEL_RULE"
        );
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM skill_resources WHERE path='references/h3-prompts.md'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        migrate(&db).unwrap();
        assert_eq!(
            db.query_row(
                "SELECT text FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![edit.skill, edit.path],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            text
        );
    }
}
