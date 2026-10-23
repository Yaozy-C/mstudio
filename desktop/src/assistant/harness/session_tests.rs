use super::*;

#[test]
fn partial_read_distinguishes_empty_saved_text_from_missing_or_failed_query() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE agent_messages(project_id TEXT, role TEXT, attribution TEXT, content TEXT); INSERT INTO agent_messages VALUES('p','assistant','{\"turnId\":\"t\"}','');").unwrap();
    assert_eq!(partial_in(&db, "p", "t").unwrap(), "");
    assert!(
        partial_in(&db, "p", "missing")
            .unwrap_err()
            .downcast_ref::<rusqlite::Error>()
            .is_some_and(|error| matches!(error, rusqlite::Error::QueryReturnedNoRows))
    );
    db.execute_batch("DROP TABLE agent_messages").unwrap();
    assert!(
        partial_in(&db, "p", "t")
            .unwrap_err()
            .to_string()
            .contains("no such table")
    );
}

#[test]
fn recovery_read_failure_does_not_invent_unknown_effect_or_partially_append() {
    let call = ToolCall::from_wire(
        "pending",
        rig_core::message::ToolFunction::new("write".try_into().unwrap(), json!({})),
    );
    let mut messages = vec![Message::Assistant(
        rig_core::message::AssistantMessage::new(vec![
            rig_core::message::AssistantContent::ToolCall(call),
        ]),
    )];
    let previous = messages.clone();
    assert_eq!(
        super::super::session_recovery::repair_pending(&mut messages, |_| Err(
            "receipt query failed".into()
        ))
        .unwrap_err(),
        "receipt query failed"
    );
    assert_eq!(messages, previous);
}
#[test]
fn native_error_flag_keeps_structured_recovery_fields_and_plain_text_success() {
    use rig_core::message::ToolFunction;
    let call = ToolCall::from_wire(
        "result",
        ToolFunction::new("write".try_into().unwrap(), json!({})),
    );
    let value = json!({"error":"Invalid arguments","issues":[{"path":"$.new_str","message":"Required"}],"recovery":{"action":"correct_arguments"}});
    let Message::User { content } = result_message(&call, &value) else {
        panic!()
    };
    let UserContent::ToolResult(result) = &content[0] else {
        panic!()
    };
    assert!(result.is_error);
    let ToolResultContent::Text(text) = &result.content[0] else {
        panic!()
    };
    assert!(text.text.contains("$.new_str") && text.text.contains("correct_arguments"));
    let Message::User { content } = result_message(&call, &json!("File saved")) else {
        panic!()
    };
    let UserContent::ToolResult(result) = &content[0] else {
        panic!()
    };
    assert!(!result.is_error);
    assert!(matches!(&result.content[0],ToolResultContent::Text(text) if text.text=="File saved"));
}
