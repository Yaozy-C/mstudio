use super::*;
fn db() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    init(&db).unwrap();
    db
}
#[test]
fn catalog_lists_descriptions_without_leaking_rule_bodies() {
    let db = db();
    seed(
        &db,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills"),
    )
    .unwrap();
    save(
        &db,
        "creative-ad-director",
        "CORE.md",
        "Current user core",
        1,
    )
    .unwrap();
    let all = catalog(&db, "").unwrap();
    assert!(!all.to_string().contains("Current user core"));
    assert!(!super::super::guidance(&all).contains("Current user core"));
    assert_eq!(
        read(&db, "", "creative-ad-director", "CORE.md", 0, true).unwrap()["text"],
        "Current user core"
    );
    assert_eq!(all.as_array().unwrap().len(), super::super::SKILLS.len());
}
#[test]
fn database_edits_survive_seed_changes_and_missing_bundle() {
    let db = db();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills");
    seed(&db, &root).unwrap();
    let page = read(&db, "", "image-production", "SKILL.md", 0, true).unwrap();
    save(
        &db,
        "image-production",
        "SKILL.md",
        "My maintained rules",
        page["revision"].as_i64().unwrap(),
    )
    .unwrap();
    seed(&db, Path::new("/missing-bundle")).unwrap();
    assert_eq!(
        read(&db, "", "image-production", "SKILL.md", 0, true).unwrap()["text"],
        "My maintained rules"
    );
    assert!(save(&db, "image-production", "SKILL.md", "stale", 1).is_err());
    assert_eq!(
        catalog(&db, "").unwrap().as_array().unwrap().len(),
        super::super::SKILLS.len()
    );
}
#[test]
fn database_reads_complete_unicode_documents_and_enforce_dependencies() {
    let db = db();
    seed(
        &db,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills"),
    )
    .unwrap();
    let text = "字".repeat(4500);
    save(&db, "image-production", "SKILL.md", &text, 1).unwrap();
    let a = read(&db, "", "image-production", "SKILL.md", 0, true).unwrap();
    let b = read(&db, "", "image-production", "SKILL.md", 4000, true).unwrap();
    assert!(a["nextOffset"].is_null());
    assert_eq!(a["text"], text);
    assert!(b["nextOffset"].is_null());
    assert_eq!(
        format!(
            "{}{}",
            &a["text"].as_str().unwrap()[..4000 * "字".len()],
            b["text"].as_str().unwrap()
        ),
        text
    );
    let enabled = "[\"skill-image-production\"]";
    assert!(
        read(
            &db,
            enabled,
            "image-production",
            "../creative-ad-director/references/photographic-appearance.md",
            0,
            true
        )
        .is_ok()
    );
    for path in ["/SKILL.md", "../../private.md", "../video-editing/SKILL.md"] {
        assert!(read(&db, enabled, "image-production", path, 0, true).is_err());
    }
    assert!(read(&db, "[]", "image-production", "SKILL.md", 0, true).is_err());
    assert!(
        save(
            &db,
            "image-production",
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
