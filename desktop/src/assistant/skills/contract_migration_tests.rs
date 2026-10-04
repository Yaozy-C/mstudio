use super::*;
fn db() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    super::super::storage::init(&db).unwrap();
    db
}
fn resource(db: &Connection, skill: &str, path: &str) -> (String, i64) {
    db.query_row(
        "SELECT text,revision FROM skill_resources WHERE skill_id=?1 AND path=?2",
        params![skill, path],
        |r| Ok((r.get(0)?, r.get(1)?)),
    )
    .unwrap()
}
fn old_resources(db: &Connection) {
    for edit in edits() {
        db.execute("INSERT INTO skill_resources(skill_id,path,text) VALUES(?1,?2,?3) ON CONFLICT(skill_id,path) DO UPDATE SET text=text||char(10)||excluded.text", params![edit.skill,edit.path,edit.before]).unwrap();
    }
}
#[test]
fn generation_lifecycle_rules_upgrade_even_when_previous_contract_version_was_applied() {
    let db = db();
    old_resources(&db);
    db.execute(
        "INSERT INTO settings VALUES('skill_contracts_v1','true')",
        [],
    )
    .unwrap();
    migrate(&db).unwrap();
    let (text, _) = resource(&db, "storyboard-art", "references/role-methods.md");
    assert!(text.contains("mstudio_await_generation"));
    assert!(!text.contains("Inspect section=generation for actual status"));
}
#[test]
fn stored_rules_upgrade_once_preserving_custom_content_and_missing_documents() {
    let db = db();
    old_resources(&db);
    db.execute(
        "UPDATE skill_resources SET text=text||char(10)||'CUSTOM_USER_RULE',revision=8",
        [],
    )
    .unwrap();
    db.execute(
        "INSERT INTO skill_resources(skill_id,path,text) VALUES('ad-team','CORE.md','CUSTOM_CORE')",
        [],
    )
    .unwrap();
    db.execute("DELETE FROM skill_resources WHERE skill_id='storyboard-art' AND path='references/role-methods.md'", []).unwrap();
    migrate(&db).unwrap();
    for edit in edits() {
        if edit.skill == "storyboard-art" && edit.path == "references/role-methods.md" {
            continue;
        }
        let (text, revision) = resource(&db, &edit.skill, &edit.path);
        assert!(text.contains(&edit.after));
        assert!(!text.contains(&edit.before));
        assert!(text.contains("CUSTOM_USER_RULE"));
        assert_eq!(revision, 9); // One revision per document, not per replacement.
    }
    assert_eq!(
        resource(&db, "ad-team", "CORE.md"),
        ("CUSTOM_CORE".into(), 1)
    );
    assert_eq!(db.query_row("SELECT count(*) FROM skill_resources WHERE skill_id='storyboard-art' AND path='references/role-methods.md'", [], |r|r.get::<_,i64>(0)).unwrap(),0);
    migrate(&db).unwrap();
    assert_eq!(resource(&db, "ad-team", "SKILL.md").1, 9);
}
#[test]
fn current_bundle_and_unrecognized_custom_rules_are_not_rewritten() {
    let db = db();
    super::super::storage::seed(
        &db,
        &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills"),
    )
    .unwrap();
    db.execute("UPDATE skill_resources SET text='USER_AUTHORED_REPLACEMENT' WHERE skill_id='ad-team' AND path='SKILL.md'", []).unwrap();
    migrate(&db).unwrap();
    assert_eq!(
        resource(&db, "ad-team", "SKILL.md"),
        ("USER_AUTHORED_REPLACEMENT".into(), 1)
    );
    let changed: i64 = db
        .query_row(
            "SELECT count(*) FROM skill_resources WHERE revision!=1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(changed, 0);
}
#[test]
fn interrupted_upgrade_rolls_back_all_rules_and_marker() {
    let db = db();
    old_resources(&db);
    let before = resource(&db, "ad-team", "SKILL.md");
    db.execute_batch("CREATE TRIGGER fail_contract_update BEFORE UPDATE ON skill_resources WHEN NEW.skill_id='product-video-production' BEGIN SELECT RAISE(ABORT,'test write failure'); END;").unwrap();
    assert!(migrate(&db).is_err());
    assert_eq!(resource(&db, "ad-team", "SKILL.md"), before);
    assert!(
        !db.query_row(
            "SELECT EXISTS(SELECT 1 FROM settings WHERE key=?1)",
            [MARKER],
            |r| r.get::<_, bool>(0)
        )
        .unwrap()
    );
    db.execute_batch("DROP TRIGGER fail_contract_update")
        .unwrap();
    migrate(&db).unwrap();
    assert_ne!(resource(&db, "ad-team", "SKILL.md"), before);
}

#[test]
fn reference_terminology_upgrades_after_v2_without_replacing_custom_rules() {
    let db = db();
    old_resources(&db);
    db.execute(
        "INSERT INTO settings VALUES('skill_contracts_v2','true')",
        [],
    )
    .unwrap();
    db.execute(
        "UPDATE skill_resources SET text=text||char(10)||'CUSTOM_USER_RULE'",
        [],
    )
    .unwrap();
    migrate(&db).unwrap();
    let (text, revision) = resource(&db, "image-prompt", "SKILL.md");
    assert!(text.contains("role selects the input mode"));
    assert!(text.contains("purpose describes"));
    assert!(text.contains("CUSTOM_USER_RULE"));
    migrate(&db).unwrap();
    assert_eq!(resource(&db, "image-prompt", "SKILL.md"), (text, revision));
}
