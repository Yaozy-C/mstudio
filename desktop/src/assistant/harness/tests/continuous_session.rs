use super::{
    super::{context_boundary, context_source, session, session_selection},
    support::*,
};
use crate::{
    assistant::{history, journal, task_context},
    database::Store,
};
use rig_core::message::{AssistantContent, Message};
use serde_json::{Value, json};

pub(super) fn fixture() -> Store {
    let root = std::env::temp_dir().join(format!("mstudio-continuous-{}", mstudio::media::id()));
    let store = Store::open(root).unwrap();
    store
        .conn()
        .unwrap()
        .execute_batch("INSERT INTO projects VALUES('p','p','{}',0),('other','other','{}',0);")
        .unwrap();
    store
}
pub(super) fn binding() -> Value {
    json!({"agentId":"coordinator","taskId":"task","provider":"openai-compatible","endpoint":"local","model":"first","revision":1})
}
pub(super) fn scope() -> task_context::Scope {
    task_context::Scope {
        task_id: "task".into(),
        agent_id: "coordinator".into(),
        targets: vec![],
        view: "script".into(),
        original_instruction: "给这个包做美区 TikTok 英文口播视频".into(),
    }
}
pub(super) fn fresh(text: &str, version: u32) -> Vec<Message> {
    vec![
        Message::System {
            content: "current role".into(),
        },
        context_source::snapshot(json!({"projectId":"p","version":version})),
        Message::user(text),
    ]
}
pub(super) fn start(store: &Store, turn: &str, binding: &Value, messages: &[Message]) {
    let request = json!({"production":{"projectId":"p","instruction":turn},"refs":[]});
    history::begin(
        store,
        "p",
        turn,
        &request,
        "test",
        &json!({"turnId":turn,"agentId":"coordinator","taskScope":scope(),"request":request}),
    )
    .unwrap();
    session::start(store, "p", turn, binding.clone(), messages).unwrap();
}
pub(super) fn append(store: &Store, turn: &str, message: Message) {
    journal::append(
        store,
        "p",
        turn,
        "session/message",
        json!({"message":message}),
    )
    .unwrap();
}
pub(super) fn cleanup(store: Store) {
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn ordinary_followups_send_the_original_goal_and_exact_tool_pairs_after_reopen() {
    let store = fixture();
    let binding = binding();
    let original = "给这个包做美区 TikTok 英文口播视频";
    start(&store, "first", &binding, &fresh(original, 1));
    let call = call("saved", "write", json!({}));
    let signed = AssistantContent::ToolCall(call.clone())
        .with_native(json!({"signature":"provider-signature"}));
    append(
        &store,
        "first",
        Message::Assistant(rig_core::message::AssistantMessage::new(vec![signed])),
    );
    let result = session::result_message(&call, &json!({"applied":true,"id":"script"}));
    append(&store, "first", result.clone());
    append(&store, "first", Message::assistant("已保存草稿"));
    history::finish(&store, "p", "first", "已保存草稿", "completed", None).unwrap();
    let restored = session_selection::for_message(&store, "p", &binding, None)
        .unwrap()
        .unwrap();
    let second = context_boundary::continue_messages(restored, &fresh("不是，就口播", 2), false);
    start(&store, "second", &binding, &second);
    append(&store, "second", Message::assistant("明白"));
    history::finish(&store, "p", "second", "明白", "completed", None).unwrap();
    let root = store.root.clone();
    drop(store);
    let store = Store::open(root).unwrap();
    let restored = session_selection::for_message(&store, "p", &binding, None)
        .unwrap()
        .unwrap();
    let third = context_boundary::continue_messages(restored, &fresh("你生成脚本啊", 2), false);
    let model = TestModel::default();
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![AssistantContent::text("完整稿")])));
    super::super::run(
        &model,
        &profile(),
        &TestHost::default(),
        session::Session::new(third),
        false,
        "",
    )
    .await
    .unwrap();
    let requests = model.requests.lock().unwrap();
    let sent = &requests[0].chat_history;
    let text = serde_json::to_string(sent).unwrap();
    for required in [
        original,
        "不是，就口播",
        "你生成脚本啊",
        "provider-signature",
    ] {
        assert_eq!(text.matches(required).count(), 1, "{required}");
    }
    assert!(sent.contains(&result));
    assert_eq!(text.matches("Current project reference data:").count(), 2);
    drop(requests);
    cleanup(store);
}

#[test]
fn compaction_and_checkpoint_are_authoritative_across_new_turns_and_model_switches() {
    let store = fixture();
    let old = binding();
    let messages = vec![
        Message::System {
            content: "old role".into(),
        },
        Message::user("old raw request"),
        Message::assistant("old raw answer"),
    ];
    start(&store, "first", &old, &messages);
    journal::append(
        &store,
        "p",
        "first",
        "session/compaction",
        json!({"start":1,"end":3,"message":Message::user("summary: English US TikTok") }),
    )
    .unwrap();
    let seq: i64 = store
        .conn()
        .unwrap()
        .query_row(
            "SELECT MAX(seq) FROM agent_events WHERE project_id='p' AND turn_id='first'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    crate::database::session_checkpoint::fold(&store.conn().unwrap(), "p", "first", seq).unwrap();
    let mut next = old.clone();
    next["model"] = json!("second");
    next["provider"] = json!("another-provider");
    next["endpoint"] = json!("new-endpoint");
    next["revision"] = json!(9);
    let restored = session_selection::for_message(&store, "p", &next, None)
        .unwrap()
        .unwrap();
    let continued = context_boundary::continue_messages(restored, &fresh("continue", 2), false);
    let text = serde_json::to_string(&continued).unwrap();
    assert!(text.contains("summary: English US TikTok"));
    assert!(!text.contains("old raw request") && !text.contains("old raw answer"));
    assert_eq!(
        continued[0],
        Message::System {
            content: "current role".into()
        }
    );
    cleanup(store);
}
