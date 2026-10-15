use crate::assistant::{
    attachment_tests,
    harness::{mailbox, session},
    profiles,
};
use serde_json::{Value, json};

#[test]
fn followups_forward_original_attachments_once_per_child_and_user_request() {
    let (root, store, _) = attachment_tests::fixture();
    let model = attachment_tests::multimodal();
    let role = profiles::builtins()
        .into_iter()
        .find(|p| p.id == "director")
        .unwrap();
    store.db.lock().unwrap().execute_batch("INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn)
        VALUES('child','p','parent','coordinator','director','continuable','running','{}','model','t');").unwrap();
    session::start(&store, "p", "t", json!({"model":"m"}), &[]).unwrap();
    let enqueue = |request: &str| {
        let source = json!({"senderAgentId":"coordinator","senderTurnId":request,"requestTurnId":request,
            "originalRequest":{"refs":[{"kind":"asset","id":"image"}],"production":{"instruction":"看这张图片"}},
            "refs":[{"kind":"asset","id":"image"}],"artifacts":[],"expectedGenerations":[]});
        store
            .db
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO subagent_inbox(child_id,text,source) VALUES('child','继续',?1)",
                [source.to_string()],
            )
            .unwrap();
    };
    let has_image = |message: &rig_core::message::Message| {
        let value = serde_json::to_value(message).unwrap();
        value["content"]
            .as_array()
            .unwrap()
            .iter()
            .any(|p| p["type"] == "image")
    };
    enqueue("first");
    enqueue("first");
    let batch = mailbox::take_inbox(&store, "child", "p", "t", &model, &role).unwrap();
    assert_eq!(batch.len(), 2);
    assert!(has_image(&batch[0]));
    assert!(!has_image(&batch[1]));
    // This metadata survives journal folding and a later child continuation.
    let through = store
        .db
        .lock()
        .unwrap()
        .query_row("SELECT max(seq) FROM agent_events", [], |r| r.get(0))
        .unwrap();
    crate::database::session_checkpoint::fold(&store.db.lock().unwrap(), "p", "t", through)
        .unwrap();
    session::start(&store, "p", "next", json!({"model":"m"}), &[]).unwrap();
    enqueue("first");
    enqueue("second");
    let next = mailbox::take_inbox(&store, "child", "p", "next", &model, &role).unwrap();
    assert!(!has_image(&next[0]));
    assert!(has_image(&next[1]));
    let sources: Vec<Value> = store
        .db
        .lock()
        .unwrap()
        .prepare("SELECT payload FROM agent_events WHERE kind='media/forwarded'")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|r| serde_json::from_str(&r.unwrap()).unwrap())
        .collect();
    assert_eq!(sources.len(), 2);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
