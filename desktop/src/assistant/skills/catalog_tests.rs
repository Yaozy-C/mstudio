use super::*;
use crate::assistant::profiles;

fn db() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    crate::assistant::skills::storage::init(&db).unwrap();
    db
}

fn disk() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills")
}

#[test]
fn consolidating_replaces_bodies_retires_skills_and_runs_once() {
    let db = db();
    // A pre-consolidation install: retired skill rows plus an edited body.
    db.execute(
        "INSERT INTO skill_resources(skill_id,path,text,revision) VALUES
         ('image-prompt','SKILL.md','OLD IMAGE PROMPT',4),
         ('ad-team','SKILL.md','OLD COORDINATION',2),
         ('ad-script','CRAFT-ONLY','STALE PATH',1)",
        [],
    )
    .unwrap();
    migrate(&db, &disk()).unwrap();
    for (skill, path) in [
        ("image-prompt", "SKILL.md"),
        ("ad-team", "SKILL.md"),
        ("ad-script", "CRAFT-ONLY"),
    ] {
        let count: i64 = db
            .query_row(
                "SELECT count(*) FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![skill, path],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(count, 0, "{skill}/{path} should be gone");
    }
    // Every shipped Skill is present with a readable entry document.
    for (id, _, _) in crate::assistant::skills::SKILLS {
        let text: String = db
            .query_row(
                "SELECT text FROM skill_resources WHERE skill_id=?1 AND path='SKILL.md'",
                [id],
                |r| r.get(0),
            )
            .unwrap_or_else(|e| panic!("{id}: {e}"));
        assert!(text.contains(id), "{id} entry names its own Skill");
    }
    let revision: i64 = db
        .query_row(
            "SELECT revision FROM skill_resources WHERE skill_id='creative-ad-director' AND path='CORE.md'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(revision, 1, "fresh shipped document starts at revision 1");
    let marker: String = db
        .query_row("SELECT value FROM settings WHERE key=?1", [MARKER], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(marker, "", "in-memory database has no backup path");
    // Running again must not bump revisions.
    migrate(&db, &disk()).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT revision FROM skill_resources WHERE skill_id='creative-ad-director' AND path='CORE.md'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        1
    );
}

#[test]
fn consolidation_keeps_shipped_agents_and_drops_retired_ones() {
    let db = db();
    let defaults = profiles::builtins();
    let mut saved = defaults.clone();
    // An edited shipped profile and two retired profiles.
    saved[0].instructions = "MY CUSTOM INSTRUCTIONS".into();
    saved[0].enabled = false;
    saved[0].skill_ids = vec!["ad-team".into(), "image-prompt".into()];
    let mut retired = defaults[0].clone();
    retired.id = "storyboard-artist".into();
    retired.skill_ids = vec!["storyboard-art".into()];
    saved.push(retired);
    let mut custom = defaults[0].clone();
    custom.id = "my-helper".into();
    custom.skill_ids = vec!["image-prompt".into(), "ad-team".into()];
    saved.push(custom);
    db.execute(
        "INSERT INTO settings(key,value) VALUES('agents',?1)",
        [serde_json::to_string(&saved).unwrap()],
    )
    .unwrap();

    migrate(&db, &disk()).unwrap();
    let next = profiles::read(&db).unwrap();
    assert!(next.iter().all(|p| p.id != "storyboard-artist"));
    let first = next.iter().find(|p| p.id == defaults[0].id).unwrap();
    assert_eq!(first.instructions, "MY CUSTOM INSTRUCTIONS");
    assert!(!first.enabled);
    assert_eq!(first.skill_ids, defaults[0].skill_ids);
    let helper = next
        .iter()
        .find(|p| p.id == "my-helper")
        .expect("user-created profiles survive");
    assert_eq!(helper.skill_ids, ["image-production"]);
}
