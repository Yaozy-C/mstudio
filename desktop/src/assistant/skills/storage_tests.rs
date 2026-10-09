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
fn database_reads_complete_unicode_documents_and_enforces_isolation() {
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
        .is_err()
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

#[test]
fn every_role_sees_only_its_bindings_and_traversal_never_crosses_a_skill_root() {
    let db = db();
    seed(
        &db,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills"),
    )
    .unwrap();
    for profile in crate::assistant::profiles::builtins() {
        let setting = crate::assistant::profiles::skill_setting(&profile);
        let visible = catalog(&db, &setting).unwrap();
        let ids: Vec<_> = visible
            .as_array()
            .unwrap()
            .iter()
            .map(|s| s["id"].as_str().unwrap())
            .collect();
        assert_eq!(ids.len(), profile.skill_ids.len(), "{}", profile.id);
        for (id, _, _) in super::super::SKILLS {
            assert_eq!(ids.contains(&id), profile.skill_ids.iter().any(|s| s == id));
            assert_eq!(
                read(&db, &setting, id, "SKILL.md", 0, true).is_ok(),
                ids.contains(&id)
            );
        }
    }
    let both = "[\"skill-image-production\",\"skill-creative-ad-director\"]";
    for setting in ["", both] {
        for path in [
            "../creative-ad-director/SKILL.md",
            "../image-production/CORE.md",
            "references/../../creative-ad-director/CORE.md",
        ] {
            assert!(read(&db, setting, "image-production", path, 0, true).is_err());
            assert!(read(&db, setting, "image-production", path, 0, false).is_err());
        }
        assert!(
            read(
                &db,
                setting,
                "image-production",
                "references/../CORE.md",
                0,
                true
            )
            .is_ok()
        );
    }
    assert_eq!(catalog(&db, "").unwrap().as_array().unwrap().len(), 6);
    assert!(catalog(&db, "[]").unwrap().as_array().unwrap().is_empty());
}
