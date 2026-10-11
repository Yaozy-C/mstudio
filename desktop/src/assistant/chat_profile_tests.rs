use super::*;

#[test]
fn stale_client_revision_uses_latest_saved_rules_and_permissions() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute(
        "CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)",
        [],
    )
    .unwrap();
    let request: Request = serde_json::from_value(json!({
        "projectId":"project", "prompt":"continue", "agentId":"coordinator",
        "agentRevision":1, "agentName":"old name", "modelName":"model",
        "clientTurnId":"turn", "messageContext":{}
    }))
    .unwrap();
    let in_flight = request.resolve_agent(&db).unwrap();
    let mut updated = in_flight.clone();
    updated.instructions = "New working instructions".into();
    updated.skill_ids = vec!["creative-ad-director".into()];
    updated.tool_ids = vec!["project-read".into()];
    profiles::save(&db, updated).unwrap();
    let next = request.resolve_agent(&db).unwrap();
    assert_eq!(next.instructions, "New working instructions");
    assert_eq!(next.skill_ids, ["creative-ad-director"]);
    assert!(!profiles::allows(&next, "edit"));
    assert!(next.revision > in_flight.revision);
    assert_ne!(in_flight.instructions, next.instructions);
    assert!(profiles::allows(&in_flight, "edit"));
    let mut disabled = next;
    disabled.enabled = false;
    profiles::save(&db, disabled).unwrap();
    assert!(request.resolve_agent(&db).is_err());
}
