use super::{permissions, profiles, tool_schema};
use serde_json::{Value, json};

#[test]
fn shipped_roles_have_independent_edit_and_generation_boundaries() {
    let agents = profiles::builtins();
    let doc = json!({"nodes":[{"id":"s","kind":"shot"},{"id":"p","kind":"screenplay"}],"production":{"drafts":{"image":{"kind":"image"},"video":{"kind":"video"}}}});
    let check = |role: &str, op: Value| {
        let agent = agents.iter().find(|a| a.id == role).unwrap();
        profiles::validate(agent).unwrap();
        let args = json!({"action":"edit","operations":[op]});
        crate::assistant::harness::schema::issues(&tool_schema::for_profile(agent), &args)
            .is_empty()
            && permissions::validate(agent, &args, &doc).is_ok()
    };
    for role in [
        "coordinator",
        "concept",
        "video-analyst",
        "writer",
        "director",
        "image",
        "production",
        "editor",
    ] {
        assert_eq!(
            check(
                role,
                json!({"op":"update_node","id":"p","screenplay":{"script":[{"id":"p1","action":"Story"}]}})
            ),
            role == "writer",
            "{role}: script"
        );
        assert_eq!(
            check(
                role,
                json!({"op":"update_node","id":"s","text":"Camera inside"})
            ),
            role == "director",
            "{role}: staging"
        );
        assert_eq!(
            check(
                role,
                json!({"op":"update_node","id":"s","shot":{"framePrompt":"Still"}})
            ),
            role == "image",
            "{role}: still prompt"
        );
        assert_eq!(
            check(
                role,
                json!({"op":"update_node","id":"s","shot":{"prompt":"Video"}})
            ),
            role == "production",
            "{role}: video prompt"
        );
        for kind in ["image", "video"] {
            let allowed = role
                == if kind == "image" {
                    "image"
                } else {
                    "production"
                };
            let mut op = json!({"op":"request_generation","mediaKind":kind,"text":"Synthetic"});
            if kind == "video" {
                op["mode"] = json!("multi");
            }
            assert_eq!(check(role, op), allowed, "{role}: {kind}");
            assert_eq!(permissions::validate(agents.iter().find(|a| a.id == role).unwrap(), &json!({"operations":[{"op":"update_generation","taskKey":kind,"text":"Changed"}]}), &doc).is_ok(), allowed, "{role}: task {kind}");
        }
    }
}

#[test]
fn existing_catalog_gains_shipped_roles_without_overwriting_custom_roles() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute(
        "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .unwrap();
    // A pre-refactor install is missing the roles profiles::read auto-adds now.
    let mut old = profiles::builtins();
    old.retain(|p| p.id == "coordinator");
    let coordinator = old.iter_mut().find(|p| p.id == "coordinator").unwrap();
    coordinator.revision = 8;
    coordinator.name = "Old default".into();
    coordinator.instructions = "My custom rules".into();
    coordinator.enabled = false;
    db.execute(
        "INSERT INTO settings VALUES('agents', ?1)",
        [serde_json::to_string(&old).unwrap()],
    )
    .unwrap();
    let loaded = profiles::read(&db).unwrap();
    assert_eq!(
        loaded.iter().filter(|p| p.id == "concept").count(),
        1,
        "an auto-added role appears exactly once"
    );
    assert_eq!(
        loaded.iter().find(|p| p.id == "coordinator").unwrap().name,
        "Old default",
        "an existing saved role is not reset to its shipped default"
    );
    for id in [
        "concept",
        "video-analyst",
        "writer",
        "director",
        "image",
        "production",
        "editor",
    ] {
        assert!(loaded.iter().any(|p| p.id == id && p.enabled));
    }
    let custom = loaded.iter().find(|p| p.id == "coordinator").unwrap();
    assert_eq!(custom.instructions, "My custom rules");
    assert!(!custom.enabled);
    let mut concept = loaded.iter().find(|p| p.id == "concept").unwrap().clone();
    concept.enabled = false;
    profiles::save(&db, concept).unwrap();
    assert!(profiles::resolve(&db, Some("concept")).is_err());
    assert_eq!(
        profiles::read(&db).unwrap().len(),
        8,
        "coordinator plus seven specialists"
    );
}

#[test]
fn saved_role_instructions_are_authoritative_even_at_default_revision() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute(
        "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .unwrap();
    let initial = profiles::read(&db).unwrap();
    let persisted: String = db
        .query_row("SELECT value FROM settings WHERE key='agents'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Vec<profiles::AgentProfile>>(&persisted)
            .unwrap()
            .len(),
        initial.len()
    );
    let mut saved = initial;
    for role in &mut saved {
        role.instructions = format!("Database rule for {}", role.id);
        role.revision = 1;
        role.enabled = false;
    }
    db.execute(
        "UPDATE settings SET value=?1 WHERE key='agents'",
        [serde_json::to_string(&saved).unwrap()],
    )
    .unwrap();
    let loaded = profiles::read(&db).unwrap();
    for (expected, actual) in saved.iter().zip(&loaded) {
        assert_eq!(actual.instructions, expected.instructions);
        assert_eq!(actual.revision, 1);
        assert!(!actual.enabled);
    }
}

#[test]
fn asset_specialist_can_register_assets_but_not_change_story_or_frame_tasks() {
    // The shipped roster has no dedicated asset role; the asset-only boundary
    // (project-assets without frames, production or edit) is kept as an inline profile.
    let mut agent = profiles::builtins()
        .into_iter()
        .find(|p| p.id == "production")
        .unwrap();
    agent.tool_ids = vec![
        "project-read".into(),
        "project-assets".into(),
        "media-generation".into(),
    ];
    profiles::validate(&agent).unwrap();
    let doc = json!({"nodes":[{"id":"asset","kind":"asset"},{"id":"shot","kind":"shot"}],
        "production":{"drafts":{"asset-task":{"kind":"image","generationPurpose":"asset"},"frame":{"kind":"image"}}}});
    let check = |op: Value| {
        let args = json!({"action":"edit","operations":[op]});
        crate::assistant::harness::schema::issues(&tool_schema::for_profile(&agent), &args)
            .is_empty()
            && permissions::validate(&agent, &args, &doc).is_ok()
    };
    assert!(check(
        json!({"op":"add_node","id":"new","kind":"asset","assetId":"image","title":"Character","text":"ready"})
    ));
    assert!(check(
        json!({"op":"update_node","id":"asset","text":"inspected"})
    ));
    assert!(check(
        json!({"op":"request_generation","mediaKind":"image","generationPurpose":"asset","references":[],"text":"Subject on white"})
    ));
    assert!(check(
        json!({"op":"update_generation","taskKey":"asset-task","text":"White background"})
    ));
    for op in [
        json!({"op":"update_generation","taskKey":"frame","text":"rewrite"}),
        json!({"op":"regenerate_generation","taskKey":"frame"}),
        json!({"op":"request_generation","mediaKind":"image"}),
        json!({"op":"request_generation","generationPurpose":"asset","mediaKind":"video"}),
        json!({"op":"request_generation","generationPurpose":"asset","mediaKind":"image","id":"shot"}),
        json!({"op":"update_node","id":"shot","shot":{"frames":[]}}),
        json!({"op":"add_node","id":"new","kind":"shot"}),
        json!({"op":"remove_node","id":"shot"}),
        json!({"op":"append_clip","assetId":"image"}),
    ] {
        assert!(!check(op.clone()), "{op}");
    }
}
