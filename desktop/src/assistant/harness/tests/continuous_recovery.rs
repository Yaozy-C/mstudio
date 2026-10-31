use super::super::{context_boundary, session, session_selection};
use super::continuous_session::{append, binding, cleanup, fixture, fresh, scope, start};
use super::support::call;
use crate::assistant::{context, history, journal, task_context};
use rig_core::message::{AssistantContent, Message};
use serde_json::json;

#[test]
fn interrupted_tools_are_closed_without_reexecution_and_resume_does_not_duplicate_input() {
    let store = fixture();
    let binding = binding();
    let original = fresh("original user request", 1);
    start(&store, "interrupted", &binding, &original);
    append(
        &store,
        "interrupted",
        Message::Assistant(rig_core::message::AssistantMessage::new(vec![
            AssistantContent::ToolCall(call("unknown", "write", json!({}))),
        ])),
    );
    let restored = session_selection::for_message(&store, "p", &binding, Some("interrupted"))
        .unwrap()
        .unwrap();
    let messages = context_boundary::continue_messages(restored, &original, true);
    let text = serde_json::to_string(&messages).unwrap();
    assert!(text.contains("EFFECT_UNKNOWN"));
    assert_eq!(text.matches("original user request").count(), 1);
    start(&store, "resumed", &binding, &messages);
    let next = session_selection::for_message(&store, "p", &binding, None)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_string(&next)
            .unwrap()
            .matches("EFFECT_UNKNOWN")
            .count(),
        1
    );
    cleanup(store);
}

#[test]
fn legacy_seed_recovers_all_user_requests_including_cancelled_turns_once() {
    let store = fixture();
    let binding = binding();
    for (turn, text, status) in [
        ("first", "给包做口播视频", "cancelled"),
        ("second", "美区 TikTok 英文", "completed"),
        ("third", "不是，就口播", "completed"),
    ] {
        let meta =
            json!({"turnId":turn,"agentId":"coordinator","taskScope":scope(),"status":status});
        history::append_attributed(&store, "p", text, &json!(text), "ack", "test", Some(&meta))
            .unwrap();
        session::start(&store, "p", turn, binding.clone(), &fresh(text, 1)).unwrap();
    }
    store.conn().unwrap().execute("UPDATE agent_events SET payload=json_remove(payload,'$.contextMode') WHERE kind='session/start'", []).unwrap();
    assert!(
        session_selection::for_message(&store, "p", &binding, None)
            .unwrap()
            .is_none()
    );
    let previous = task_context::history(&store, "p", &scope(), "current").unwrap();
    let recalled =
        context::assemble_with_budget(&previous, json!("现在出稿"), json!({}), 1).unwrap();
    let text = recalled.to_string();
    for required in ["给包做口播视频", "美区 TikTok 英文", "不是，就口播"] {
        assert_eq!(text.matches(required).count(), 1, "{required}");
    }
    let messages: Vec<_> = recalled
        .as_array()
        .unwrap()
        .iter()
        .map(crate::assistant::agent::convert)
        .collect();
    start(&store, "current", &binding, &messages);
    assert_eq!(
        task_context::history(&store, "p", &scope(), "current")
            .unwrap()
            .len(),
        previous.len()
    );
    let restored = session_selection::for_message(&store, "p", &binding, None)
        .unwrap()
        .unwrap();
    assert_eq!(restored, messages);
    cleanup(store);
}

#[test]
fn project_role_task_reset_and_archive_boundaries_never_select_an_older_session() {
    let store = fixture();
    let binding = binding();
    start(&store, "first", &binding, &fresh("project p only", 1));
    assert!(
        session_selection::for_message(&store, "other", &binding, None)
            .unwrap()
            .is_none()
    );
    for key in ["agentId", "taskId"] {
        let mut other = binding.clone();
        other[key] = json!("other");
        assert!(
            session_selection::for_message(&store, "p", &other, None)
                .unwrap()
                .is_none()
        );
    }
    let mut archived = binding.clone();
    archived["archiveContext"] = json!({"archived":["old-job"]});
    assert!(
        session_selection::for_message(&store, "p", &archived, None)
            .unwrap()
            .is_none()
    );
    journal::append(&store, "p", "reset", "session/reset", json!({})).unwrap();
    assert!(
        session_selection::for_message(&store, "p", &binding, None)
            .unwrap()
            .is_none()
    );
    cleanup(store);
}
