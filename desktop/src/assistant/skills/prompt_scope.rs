//! One-time configuration migration; runtime behavior is driven by Skill assignments.
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
pub fn install(db: &Connection) -> Result<()> {
    if db.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key='scoped_prompt_skills_v1')",
        [],
        |r| r.get::<_, bool>(0),
    )? {
        return Ok(());
    }
    let backup = if let Some(path) = db.path().filter(|p| !p.is_empty()) {
        let dest = format!(
            "{path}.before-scoped-prompt-skills-{}.sqlite3",
            mstudio::media::id()
        );
        db.execute("VACUUM INTO ?1", [&dest])?;
        dest
    } else {
        String::new()
    };
    let tx = db.unchecked_transaction()?;
    for (id, text) in [
        (
            "image-prompt",
            include_str!("../../../../skills/image-prompt/SKILL.md"),
        ),
        (
            "video-prompt",
            include_str!("../../../../skills/video-prompt/SKILL.md"),
        ),
    ] {
        tx.execute(
            "INSERT OR IGNORE INTO skill_resources(skill_id,path,text) VALUES(?1,'SKILL.md',?2)",
            params![id, text],
        )?;
    }
    let edits: Vec<Edit> = serde_json::from_str(include_str!("prompt_scope_migration.json"))?;
    for edit in edits {
        let old: Option<String> = tx
            .query_row(
                "SELECT text FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![edit.skill, edit.path],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(old) = old {
            let new = old.replace(&edit.before, &edit.after);
            if new != old {
                tx.execute("UPDATE skill_resources SET text=?3,revision=revision+1,updated=unixepoch() WHERE skill_id=?1 AND path=?2",params![edit.skill,edit.path,new])?;
            }
        }
    }
    let mut agents = super::super::profiles::read(&tx)?;
    let defaults = super::super::profiles::builtins();
    for agent in &mut agents {
        if let Some(default) = defaults.iter().find(|a| a.id == agent.id)
            && agent.skill_ids != default.skill_ids
        {
            agent.skill_ids.clone_from(&default.skill_ids);
            agent.revision += 1;
        }
    }
    tx.execute(
        "UPDATE settings SET value=?1 WHERE key='agents'",
        [serde_json::to_string(&agents)?],
    )?;
    tx.execute(
        "INSERT INTO settings VALUES('scoped_prompt_skills_v1',?1)",
        [backup],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn saved_agent_assignments_are_scoped_and_other_settings_survive() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            .unwrap();
        super::super::storage::seed(
            &db,
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills"),
        )
        .unwrap();
        let mut agents = super::super::super::profiles::builtins();
        for a in &mut agents {
            a.skill_ids = vec!["creative-ad-director".into()];
            a.instructions = "MY CUSTOM INSTRUCTIONS".into();
        }
        agents[0].enabled = false;
        db.execute(
            "INSERT INTO settings VALUES('agents',?1)",
            [serde_json::to_string(&agents).unwrap()],
        )
        .unwrap();
        install(&db).unwrap();
        let saved = super::super::super::profiles::read(&db).unwrap();
        for a in &saved {
            assert_eq!(a.instructions, "MY CUSTOM INSTRUCTIONS");
            let prompt_skills: Vec<_> = a
                .skill_ids
                .iter()
                .filter(|s| s.ends_with("-prompt"))
                .map(String::as_str)
                .collect();
            match a.id.as_str() {
                "asset-designer" | "storyboard-artist" => {
                    assert_eq!(prompt_skills, vec!["image-prompt"])
                }
                "production" => assert_eq!(prompt_skills, vec!["video-prompt"]),
                _ => assert!(prompt_skills.is_empty()),
            }
        }
        assert!(!saved[0].enabled);
        db.execute(
            "UPDATE settings SET value=?1 WHERE key='agents'",
            [serde_json::to_string(&agents).unwrap()],
        )
        .unwrap();
        install(&db).unwrap();
        assert_eq!(
            super::super::super::profiles::read(&db).unwrap()[0].skill_ids,
            agents[0].skill_ids
        );
    }
}
