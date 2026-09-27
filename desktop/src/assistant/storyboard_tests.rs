use super::{permissions, profiles, tool_schema};
use serde_json::json;

#[test]
fn artist_can_draw_but_cannot_rewrite_shots_or_generate_video() {
    let artist = profiles::builtins()
        .into_iter()
        .find(|p| p.id == "storyboard-artist")
        .unwrap();
    profiles::validate(&artist).unwrap();
    let doc = json!({"nodes":[{"id":"s","kind":"shot"},{"id":"p","kind":"plan"}]});
    let check = |op| permissions::validate(&artist, &json!({"operations":[op]}), &doc).is_ok();
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
        json!({"op":"update_node","id":"p","plan":{"story":"New story"}}),
        json!({"op":"remove_node","id":"s"}),
        json!({"op":"add_node","id":"new","kind":"shot"}),
    ] {
        assert!(!check(op));
    }
    assert_eq!(
        tool_schema::for_profile(&artist)["properties"]["operations"]["items"]["properties"]["mediaKind"]
            ["enum"],
        json!(["image"])
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
        !["storyboard-artist", "colorist", "transition-designer"].contains(&p.id.as_str())
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
        "分镜导演"
    );
    for id in ["colorist", "transition-designer"] {
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
    assert_eq!(profiles::read(&db).unwrap().len(), 9);
}
