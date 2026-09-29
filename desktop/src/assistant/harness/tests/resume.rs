use super::*;
#[test]
fn interrupted_calls_get_unknown_effect_result_never_reexecuted() {
    let tool = call("c", "write", json!({}));
    let mut messages = vec![
        Message::user("request"),
        Message::Assistant {
            id: None,
            content: vec![AssistantContent::ToolCall(tool)],
        },
    ];
    session::repair_pending(&mut messages);
    assert_eq!(messages.len(), 3);
    assert!(
        serde_json::to_string(&messages[2])
            .unwrap()
            .contains("EFFECT_UNKNOWN")
    );
    session::repair_pending(&mut messages);
    assert_eq!(messages.len(), 3);
}
#[test]
fn resetting_session_invalidates_previous_resume() {
    let store = store();
    let binding = json!({"model":"a"});
    session::start(&store, "p", "t", binding.clone(), &[Message::user("old")]).unwrap();
    journal::append(&store, "p", "reset", "session/reset", json!({})).unwrap();
    assert!(session::restore(&store, "p", "t", &binding).is_err());
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn resume_requires_same_request_and_interrupted_turn() {
    use crate::assistant::history;
    let store = store();
    let request = json!({"production":{"projectId":"p","instruction":"edit"},"refs":[]});
    let meta = json!({"turnId":"turn","request":request});
    history::begin(&store, "p", "turn", &request, "test", &meta).unwrap();
    assert!(session::validate_resume(&store, "p", "turn", &request).is_err());
    journal::append(
        &store,
        "p",
        "turn",
        "assistant/partial",
        json!({"text":"prefix","delta":"prefix"}),
    )
    .unwrap();
    assert_eq!(session::partial(&store, "p", "turn"), "prefix");
    store.db.lock().unwrap().execute("UPDATE agent_messages SET attribution=json_set(attribution,'$.status','failed') WHERE role='assistant'",[]).unwrap();
    session::validate_resume(&store, "p", "turn", &request).unwrap();
    assert!(session::validate_resume(&store, "other", "turn", &request).is_err());
    assert!(session::validate_resume(&store, "p", "turn", &json!({"changed":true})).is_err());
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
