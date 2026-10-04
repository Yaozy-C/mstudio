use super::{permissions, profiles, tool_schema};
use serde_json::{Value, json};

#[test]
fn artist_can_draw_but_cannot_rewrite_shots_or_generate_video() {
    let artist = profiles::builtins()
        .into_iter()
        .find(|p| p.id == "storyboard-artist")
        .unwrap();
    profiles::validate(&artist).unwrap();
    let doc = json!({"nodes":[{"id":"s","kind":"shot"},{"id":"p","kind":"screenplay"}]});
    let check = |op: Value| {
        let args = json!({"action":"edit","operations":[op]});
        crate::assistant::harness::schema::issues(&tool_schema::for_profile(&artist), &args)
            .is_empty()
            && permissions::validate(&artist, &args, &doc).is_ok()
    };
    assert!(check(
        json!({"op":"update_node","id":"s","shot":{"framePrompt":"Wide shot","frames":[{"assetId":"image","title":"S01-A"}]}})
    ));
    assert!(check(
        json!({"op":"request_generation","id":"s","mediaKind":"image","text":"Wide shot"})
    ));
    for op in [
        json!({"op":"request_generation","mediaKind":"video"}),
        json!({"op":"request_generation"}),
        json!({"op":"update_node","id":"s","text":"Change action"}),
        json!({"op":"update_node","id":"s","shot":{"duration":2}}),
        json!({"op":"update_node","id":"s","shot":{"prompt":"Video"}}),
        json!({"op":"update_node","id":"p","screenplay":{"story":"New story"}}),
        json!({"op":"remove_node","id":"s"}),
        json!({"op":"add_node","id":"new","kind":"shot"}),
    ] {
        assert!(!check(op));
    }
    assert_eq!(
        tool_schema::for_profile(&artist)["properties"]["operations"]["items"]["oneOf"]
            .as_array()
            .unwrap()
            .iter()
            .find(|s| s["properties"]["op"]["const"] == "request_generation")
            .unwrap()["properties"]["mediaKind"]["const"],
        json!("image")
    );
    let mut revoked = artist;
    revoked.tool_ids.retain(|id| id != "media-generation");
    assert!(!permissions::allows_operation(
        &revoked,
        "request_generation"
    ));
}

#[test]
fn existing_catalog_gains_artist_without_overwriting_custom_roles() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute(
        "CREATE TABLE settings (key TEXT PRIMARY KEY, value TEXT NOT NULL)",
        [],
    )
    .unwrap();
    let mut old = profiles::builtins();
    old.retain(|p| {
        ![
            "storyboard-artist",
            "colorist",
            "transition-designer",
            "asset-designer",
        ]
        .contains(&p.id.as_str())
    });
    let storyboard = old.iter_mut().find(|p| p.id == "storyboard").unwrap();
    storyboard.revision = 1;
    storyboard.name = "Old default".into();
    let production = old.iter_mut().find(|p| p.id == "production").unwrap();
    production.revision = 8;
    production.instructions = "My custom rules".into();
    production.enabled = false;
    db.execute(
        "INSERT INTO settings VALUES('agents', ?1)",
        [serde_json::to_string(&old).unwrap()],
    )
    .unwrap();
    let loaded = profiles::read(&db).unwrap();
    assert_eq!(
        loaded
            .iter()
            .filter(|p| p.id == "storyboard-artist")
            .count(),
        1
    );
    assert_eq!(
        loaded.iter().find(|p| p.id == "storyboard").unwrap().name,
        "Old default"
    );
    for id in ["colorist", "transition-designer", "asset-designer"] {
        assert!(loaded.iter().any(|p| p.id == id && p.enabled));
    }
    let custom = loaded.iter().find(|p| p.id == "production").unwrap();
    assert_eq!(custom.instructions, "My custom rules");
    assert!(!custom.enabled);
    let mut artist = loaded
        .iter()
        .find(|p| p.id == "storyboard-artist")
        .unwrap()
        .clone();
    artist.enabled = false;
    profiles::save(&db, artist).unwrap();
    assert!(profiles::resolve(&db, Some("storyboard-artist")).is_err());
    assert_eq!(profiles::read(&db).unwrap().len(), 10);
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
    let agent = profiles::builtins()
        .into_iter()
        .find(|p| p.id == "asset-designer")
        .unwrap();
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
        assert!(!check(op));
    }
}
