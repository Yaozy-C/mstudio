use super::{super::session, support::call};
use crate::{assistant::journal, database::Store};
use rig_core::message::{
    AssistantContent, ImageMediaType, Message, ToolResultContent, UserContent,
};
use serde_json::json;
fn store() -> Store {
    let root = std::env::temp_dir().join(format!("mstudio-harness-{}", mstudio::media::id()));
    let store = Store::open(root).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','test','{}',0)", [])
        .unwrap();
    store
}
#[test]
fn context_pressure_offloads_old_images_and_restores_replacement() {
    let image = match UserContent::image_base64("aGVsbG8=", Some(ImageMediaType::PNG), None) {
        UserContent::Image(image) => image,
        _ => unreachable!(),
    };
    let tool = call("c", "inspect", json!({}));
    let mut result = session::result_message(&tool, &json!({"ok": true}));
    if let Message::User { content } = &mut result
        && let UserContent::ToolResult(value) = &mut content[0]
    {
        value.content.push(ToolResultContent::Image(image.clone()));
    }
    let messages = vec![
        Message::System {
            content: "rules".into(),
        },
        Message::User {
            content: vec![UserContent::text("旧请求"), UserContent::Image(image)],
        },
        Message::Assistant {
            id: None,
            content: vec![AssistantContent::ToolCall(tool)],
        },
        result,
        Message::Assistant {
            id: None,
            content: vec![AssistantContent::text("images reviewed")],
        },
        Message::user("new request"),
        Message::user("current attachment"),
    ];
    let store = store();
    let binding = json!({"model":"a"});
    session::start(&store, "p", "t", binding.clone(), &messages).unwrap();
    struct RecordingHost<'a>(&'a Store);
    impl crate::assistant::harness::Host for RecordingHost<'_> {
        fn token(&self) -> &tokio_util::sync::CancellationToken {
            static TOKEN: std::sync::OnceLock<tokio_util::sync::CancellationToken> =
                std::sync::OnceLock::new();
            TOKEN.get_or_init(tokio_util::sync::CancellationToken::new)
        }
        fn definitions(&self) -> Vec<rig_core::completion::ToolDefinition> {
            vec![]
        }
        fn parallel_safe(&self, _: &rig_core::message::ToolCall) -> bool {
            false
        }
        fn record(&self, kind: &str, value: serde_json::Value) -> Result<(), String> {
            journal::append(self.0, "p", "t", kind, value).map_err(|e| e.to_string())
        }
        async fn execute(&self, _: &rig_core::message::ToolCall) -> serde_json::Value {
            json!({})
        }
    }
    let mut session = session::Session::new(messages);
    assert_eq!(
        session::offload_old_images(&mut session, &RecordingHost(&store)).unwrap(),
        2
    );
    let restored = session::restore(&store, "p", "t", &binding)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&restored).unwrap(),
        serde_json::to_value(&session.messages).unwrap()
    );
    let serialized = serde_json::to_string(&session.messages).unwrap();
    assert!(serialized.contains("旧请求"));
    assert!(serialized.contains("inspect"));
    assert!(serialized.contains("ok"));
    assert!(serialized.contains("已卸载"));
    assert!(!serialized.contains("aGVsbG8="));
    let (id,image):(String,String)=store.db.lock().unwrap().query_row(
        "SELECT json_extract(j.value,'$.id'),json_extract(j.value,'$.image') FROM agent_events e,json_each(e.payload,'$.offloads') j WHERE e.project_id='p' AND e.kind='image/offload' LIMIT 1",[],|r|Ok((r.get(0)?,r.get(1)?)),
    ).unwrap();
    assert!(serialized.contains(&id));
    let typed = session::result_message(
        &call("reopen", "mstudio_reopen_image", json!({"imageId":id})),
        &json!({"imageId":id,"__offloadedImage":serde_json::from_str::<serde_json::Value>(&image).unwrap()}),
    );
    assert!(serde_json::to_string(&typed).unwrap().contains("aGVsbG8="));
    assert!(
        !serde_json::to_string(&journal::page(&store, "p", None).unwrap())
            .unwrap()
            .contains("aGVsbG8=")
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn replay_preserves_provider_signatures_and_results_but_ui_does_not_expose_them() {
    let store = store();
    let binding = json!({"model":"a"});
    session::start(
        &store,
        "p",
        "t",
        binding.clone(),
        &[Message::user("request")],
    )
    .unwrap();
    let mut tool = call("c", "write", json!({}));
    tool.signature = Some("private-provider-signature".into());
    let message = Message::Assistant {
        id: Some("msg-1".into()),
        content: vec![AssistantContent::ToolCall(tool.clone())],
    };
    journal::append(
        &store,
        "p",
        "t",
        "session/message",
        json!({"message":message}),
    )
    .unwrap();
    let result = session::result_message(&tool, &json!({"applied":true}));
    journal::append(
        &store,
        "p",
        "t",
        "tool/result",
        json!({"callId":"c","result":{"applied":true},"message":result}),
    )
    .unwrap();
    let restored = session::restore(&store, "p", "t", &binding)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&restored[1]).unwrap(),
        serde_json::to_value(message).unwrap()
    );
    assert_eq!(
        serde_json::to_value(&restored[2]).unwrap(),
        serde_json::to_value(result).unwrap()
    );
    assert_eq!(restored.len(), 3);
    assert!(
        !serde_json::to_string(&journal::turn_page(&store, "p", "t", None).unwrap())
            .unwrap()
            .contains("private-provider-signature")
    );
    assert!(session::restore(&store, "p", "t", &json!({"model":"changed"})).is_err());
    assert!(
        session::restore(&store, "other", "t", &binding)
            .unwrap()
            .is_none()
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn replay_applies_durable_compaction_replacement() {
    let store = store();
    let binding = json!({"model":"a"});
    session::start(
        &store,
        "p",
        "t",
        binding.clone(),
        &[
            Message::System {
                content: "rules".into(),
            },
            Message::user("old"),
            Message::user("latest"),
        ],
    )
    .unwrap();
    journal::append(
        &store,
        "p",
        "t",
        "session/compaction",
        json!({"start":1,"end":2,"message":Message::user("summary")}),
    )
    .unwrap();
    let restored = session::restore(&store, "p", "t", &binding)
        .unwrap()
        .unwrap();
    let text = serde_json::to_string(&restored).unwrap();
    assert!(text.contains("summary"));
    assert!(text.contains("latest"));
    assert!(!text.contains("old"));
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn parent_history_ignores_child_session_starts() {
    use crate::assistant::history;
    let store = store();
    let binding = json!({"model":"a"});
    let request = json!({"production":{"projectId":"p","instruction":"hello"},"refs":[]});
    history::begin(
        &store,
        "p",
        "turn",
        &request,
        "test",
        &json!({"turnId":"turn","request":request}),
    )
    .unwrap();
    session::start(
        &store,
        "p",
        "turn",
        binding.clone(),
        &[Message::user("parent context")],
    )
    .unwrap();
    session::start(
        &store,
        "p",
        "child",
        json!({"model":"child"}),
        &[Message::user("child context")],
    )
    .unwrap();
    let latest = super::super::session_selection::latest(&store, "p", &binding)
        .unwrap()
        .unwrap();
    assert!(
        serde_json::to_string(&latest)
            .unwrap()
            .contains("parent context")
    );
    assert!(
        !serde_json::to_string(&latest)
            .unwrap()
            .contains("child context")
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
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
