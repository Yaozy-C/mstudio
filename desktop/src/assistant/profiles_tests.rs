use super::*;
#[test]
fn agent_skills_are_scoped_and_edit_requires_read() {
    let mut p = defaults("");
    p.skill_ids = vec!["creative-ad-director".into()];
    p.tool_ids.clear();
    assert!(!allows(&p, "edit"));
    assert!(!allows(&p, "inspect"));
    assert_eq!(skill_setting(&p), "[\"skill-creative-ad-director\"]");
    p.tool_ids.push("project-edit".into());
    assert!(validate(&p).is_err());
    p.tool_ids.push("project-read".into());
    assert!(validate(&p).is_ok());
    assert!(allows(&p, "edit"));
}
#[test]
fn tools_and_skills_are_independent_and_revocation_changes_schema() {
    let mut p = builtins()
        .into_iter()
        .find(|p| p.id == "production")
        .unwrap();
    p.tool_ids.clear();
    assert!(allows(&p, "read_skill"));
    assert!(!allows(&p, "inspect"));
    assert!(!allows(&p, "edit"));
    assert!(!super::super::permissions::allows_operation(
        &p,
        "request_generation"
    ));
    let schema = super::super::tool_schema::for_profile(&p);
    assert_eq!(
        schema["properties"]["action"]["enum"],
        serde_json::json!(["skills", "read_skill"])
    );
    p.skill_ids.clear();
    p.tool_ids = vec!["project-read".into(), "project-script".into()];
    assert!(allows(&p, "edit"));
    assert!(!allows(&p, "read_skill"));
    p.skill_ids.push("project-read".into());
    assert!(validate(&p).is_err());
}
#[test]
fn agent_profiles_persist_independently_and_revision_changes() {
    let root = std::env::temp_dir().join(format!("mstudio-agent-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    let db = store.db.lock().unwrap();
    let mut first = defaults("");
    first.instructions = "Creative direction".into();
    save(&db, first.clone()).unwrap();
    let mut other = first.clone();
    other.id = "other".into();
    other.skill_ids.clear();
    save(&db, other).unwrap();
    assert_eq!(read(&db).unwrap().len(), builtins().len() + 1);
    assert_eq!(resolve(&db, None).unwrap().revision, first.revision + 1);
    assert!(resolve(&db, Some("other")).is_ok());
    first.enabled = false;
    save(&db, first).unwrap();
    assert!(resolve(&db, None).is_err());
    drop(db);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn media_read_is_read_only_and_requires_project_read() {
    let mut profile = defaults("");
    profile.tool_ids = vec!["media-read".into()];
    assert!(validate(&profile).is_err());
    assert!(!allows(&profile, "read_media"));
    profile.tool_ids.push("project-read".into());
    assert!(validate(&profile).is_ok());
    assert!(allows(&profile, "read_media"));
    assert!(!allows(&profile, "edit"));
}

#[test]
fn coordinator_delegates_without_media_inspection_or_production_authoring() {
    let profile = defaults("");
    assert!(!allows(&profile, "read_media"));
    assert!(allows(&profile, "inspect"));
    assert!(allows(&profile, "delegate"));
    for operation in ["request_generation", "set_frame", "append_clip"] {
        assert!(!super::super::permissions::allows_operation(
            &profile, operation
        ));
    }
    for specialist in builtins().into_iter().filter(|p| {
        matches!(
            p.id.as_str(),
            "concept" | "writer" | "director" | "assets" | "image" | "production" | "editor"
        )
    }) {
        assert!(allows(&specialist, "read_media"), "{}", specialist.id);
    }
}

#[test]
fn material_tool_upgrade_preserves_customization_and_later_tool_removal() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    let mut old = builtins();
    for role in &mut old {
        role.tool_ids.retain(|id| id != "material-read");
        role.instructions = format!("Custom {}", role.id);
    }
    db.execute(
        "INSERT INTO settings VALUES('agents',?1)",
        [serde_json::to_string(&old).unwrap()],
    )
    .unwrap();
    let upgraded = read(&db).unwrap();
    for role in &upgraded {
        assert_eq!(role.instructions, format!("Custom {}", role.id));
        assert_eq!(
            role.tool_ids.iter().any(|id| id == "material-read"),
            matches!(role.id.as_str(), "concept" | "assets" | "writer")
        );
    }
    let mut writer = upgraded.into_iter().find(|a| a.id == "writer").unwrap();
    writer.tool_ids.retain(|id| id != "material-read");
    save(&db, writer).unwrap();
    assert!(
        !resolve(&db, Some("writer"))
            .unwrap()
            .tool_ids
            .iter()
            .any(|id| id == "material-read")
    );
}
