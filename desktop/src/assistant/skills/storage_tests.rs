use super::*;
fn db() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    init(&db).unwrap();
    db
}
#[test]
fn cores_follow_enabled_database_rules_and_preserve_user_edits() {
    let db = db();
    seed(
        &db,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills"),
    )
    .unwrap();
    install_core_defaults(&db).unwrap();
    save(&db, "storyboard-art", "CORE.md", "Current user core", 1).unwrap();
    install_core_defaults(&db).unwrap();
    let all = catalog(&db, "").unwrap();
    assert_eq!(
        all.as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == "storyboard-art")
            .unwrap()["core"],
        "Current user core"
    );
    let guidance = super::super::guidance(&all);
    assert_eq!(guidance.matches("Current user core").count(), 1);
    assert!(
        !guidance
            .contains("Inspect original product images before describing structural relationships")
    );
    let disabled = catalog(&db, "[]").unwrap();
    assert!(
        disabled
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == "storyboard-art")
            .unwrap()["core"]
            .is_null()
    );
}
#[test]
fn new_asset_skill_installs_once_without_overwriting_database_edits() {
    let db = db();
    db.execute(
        "INSERT INTO settings VALUES('skills_initialized','true')",
        [],
    )
    .unwrap();
    install_asset_defaults(&db).unwrap();
    let page = read(&db, "", "asset-preparation", "SKILL.md", 0, true).unwrap();
    save(
        &db,
        "asset-preparation",
        "SKILL.md",
        "User rules",
        page["revision"].as_i64().unwrap(),
    )
    .unwrap();
    install_asset_defaults(&db).unwrap();
    assert_eq!(
        read(&db, "", "asset-preparation", "SKILL.md", 0, true).unwrap()["text"],
        "User rules"
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM skill_resources", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn database_edits_survive_seed_changes_and_missing_bundle() {
    let db = db();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills");
    seed(&db, &root).unwrap();
    let page = read(&db, "", "storyboard-art", "SKILL.md", 0, true).unwrap();
    save(
        &db,
        "storyboard-art",
        "SKILL.md",
        "My maintained rules",
        page["revision"].as_i64().unwrap(),
    )
    .unwrap();
    seed(&db, Path::new("/missing-bundle")).unwrap();
    assert_eq!(
        read(&db, "", "storyboard-art", "SKILL.md", 0, true).unwrap()["text"],
        "My maintained rules"
    );
    assert!(save(&db, "storyboard-art", "SKILL.md", "stale", 1).is_err());
    assert_eq!(
        catalog(&db, "").unwrap().as_array().unwrap().len(),
        super::super::SKILLS.len()
    );
}
#[test]
fn database_reads_paginate_unicode_and_enforce_dependencies() {
    let db = db();
    seed(
        &db,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills"),
    )
    .unwrap();
    let text = "字".repeat(4500);
    save(&db, "storyboard-art", "SKILL.md", &text, 1).unwrap();
    let a = read(&db, "", "storyboard-art", "SKILL.md", 0, true).unwrap();
    let b = read(&db, "", "storyboard-art", "SKILL.md", 4000, true).unwrap();
    assert_eq!(a["nextOffset"], 4000);
    assert!(b["nextOffset"].is_null());
    assert_eq!(
        format!(
            "{}{}",
            a["text"].as_str().unwrap(),
            b["text"].as_str().unwrap()
        ),
        text
    );
    let enabled = "[\"skill-storyboard-art\"]";
    assert!(
        read(
            &db,
            enabled,
            "storyboard-art",
            "../creative-ad-director/references/image-prompt-writing.md",
            0,
            true
        )
        .is_ok()
    );
    for path in ["/SKILL.md", "../../private.md", "../color-grading/SKILL.md"] {
        assert!(read(&db, enabled, "storyboard-art", path, 0, true).is_err());
    }
    assert!(read(&db, "[]", "storyboard-art", "SKILL.md", 0, true).is_err());
    assert!(
        save(
            &db,
            "storyboard-art",
            "../creative-ad-director/SKILL.md",
            "other",
            1
        )
        .is_err()
    );
}
#[test]
fn failed_seed_does_not_leave_partial_defaults() {
    let db = db();
    assert!(seed(&db, Path::new("/missing-bundle")).is_err());
    assert!(!initialized(&db).unwrap());
    assert_eq!(
        db.query_row("SELECT count(*) FROM skill_resources", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
