use super::*;
use crate::assistant::{harness::session, history, journal};
use rig_core::message::Message;

fn store() -> Store {
    let root = std::env::temp_dir().join(format!("mstudio-history-{}", mstudio::media::id()));
    let store = Store::open(root).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','p','{}',0)", [])
        .unwrap();
    store
}
fn emit(store: &Store, turn: &str, kind: &str, value: Value) {
    journal::append(store, "p", turn, kind, value).unwrap();
}
fn count(db: &Connection, kind: &str) -> i64 {
    db.query_row(
        "SELECT count(*) FROM agent_events WHERE kind=?1",
        [kind],
        |r| r.get(0),
    )
    .unwrap()
}
fn dispose(store: Store) {
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn snapshot_preserves_rewrites_and_retries_without_duplicate_results() {
    let store = store();
    let binding = json!({"agentId":"coordinator","model":"m"});
    session::start(
        &store,
        "p",
        "t",
        binding.clone(),
        &[Message::user("first"), Message::assistant("old")],
    )
    .unwrap();
    emit(
        &store,
        "t",
        "session/compaction",
        json!({"start":0,"end":2,"message":Message::user("summary")}),
    );
    emit(
        &store,
        "t",
        "tool/result",
        json!({"callId":"c","value":{"ok":true},"message":Message::user("result")}),
    );
    emit(
        &store,
        "t",
        "session/message",
        json!({"message":Message::assistant("answer")}),
    );
    let before =
        serde_json::to_value(session::restore(&store, "p", "t", &binding).unwrap()).unwrap();
    emit(&store, "t", "turn/end", json!({"status":"failed"}));
    assert_eq!(
        serde_json::to_value(session::restore(&store, "p", "t", &binding).unwrap()).unwrap(),
        before
    );
    assert_eq!(count(&store.db.lock().unwrap(), "session/message"), 0);
    assert_eq!(count(&store.db.lock().unwrap(), "session/compaction"), 0);
    // Resume with the same turn: a new start supersedes the old snapshot at settlement.
    let restored = session::restore(&store, "p", "t", &binding)
        .unwrap()
        .unwrap();
    session::start(&store, "p", "t", binding.clone(), &restored).unwrap();
    emit(
        &store,
        "t",
        "session/message",
        json!({"message":Message::assistant("retry answer")}),
    );
    emit(&store, "t", "turn/end", json!({"status":"completed"}));
    let after = session::restore(&store, "p", "t", &binding)
        .unwrap()
        .unwrap();
    assert_eq!(after.len(), restored.len() + 1);
    assert_eq!(count(&store.db.lock().unwrap(), "session/start"), 1);
    {
        let db = store.db.lock().unwrap();
        history_cleanup::run(&db, 0).unwrap();
        history_cleanup::run(&db, 0).unwrap();
    }
    assert_eq!(
        session::restore(&store, "p", "t", &binding)
            .unwrap()
            .unwrap()
            .len(),
        after.len()
    );
    dispose(store);
}
#[test]
fn expiry_keeps_active_turns_and_referenced_results_and_usage_totals() {
    let store = store();
    history::begin(
        &store,
        "p",
        "active",
        &json!({"production":{"projectId":"p","instruction":"work"},"refs":[]}),
        "m",
        &json!({"turnId":"active"}),
    )
    .unwrap();
    for turn in ["old", "recent", "active"] {
        emit(
            &store,
            turn,
            "tool/call",
            json!({"callId":format!("{turn}-call")}),
        );
        emit(
            &store,
            turn,
            "tool/result",
            json!({"callId":format!("{turn}-call"),"value":{"ok":true}}),
        );
        if turn != "active" {
            emit(&store, turn, "turn/end", json!({"status":"completed"}));
        }
    }
    session::start(
        &store,
        "p",
        "recover",
        json!({}),
        &[Message::user(r#"{"resultRef":"old-call","turnId":"old"}"#)],
    )
    .unwrap();
    emit(&store, "recover", "turn/end", json!({"status":"failed"}));
    for _ in 0..3 {
        emit(
            &store,
            "old",
            "request/usage",
            json!({"inputTokens":10,"outputTokens":2,"totalTokens":12,"cachedInputTokens":4}),
        );
    }
    emit(
        &store,
        "old",
        "compaction/end",
        json!({"inputTokens":5,"outputTokens":1,"totalTokens":6}),
    );
    let db = store.db.lock().unwrap();
    db.execute(
        "UPDATE agent_events SET created=1 WHERE turn_id!='recent'",
        [],
    )
    .unwrap();
    history_cleanup::run(&db, 7 * 24 * 60 * 60 + 2).unwrap();
    assert_eq!(count(&db, "tool/call"), 2); // active + recent
    assert_eq!(count(&db, "tool/result"), 3); // old result is a recovery dependency
    assert_eq!(count(&db, "request/usage"), 0);
    let raw: String = db
        .query_row(
            "SELECT payload FROM agent_events WHERE kind='turn/usage'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let usage: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(
        usage,
        json!({"requests":3,"compactions":1,"inputTokens":35,"outputTokens":7,"totalTokens":42,"cachedInputTokens":12})
    );
    db.execute("DELETE FROM agent_events WHERE turn_id='recover'", [])
        .unwrap();
    history_cleanup::run(&db, 7 * 24 * 60 * 60 + 2).unwrap();
    assert_eq!(count(&db, "tool/result"), 2);
    drop(db);
    dispose(store);
}
#[test]
fn malformed_compaction_rolls_back_without_destroying_recovery_data() {
    let store = store();
    session::start(&store, "p", "t", json!({}), &[Message::user("keep")]).unwrap();
    emit(
        &store,
        "t",
        "session/compaction",
        json!({"start":0,"end":999,"message":Message::user("bad")}),
    );
    let db = store.db.lock().unwrap();
    assert!(history_cleanup::run(&db, 0).is_err());
    assert_eq!(count(&db, "session/compaction"), 1);
    let raw: String = db
        .query_row(
            "SELECT payload FROM agent_events WHERE kind='session/start'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(raw.contains("keep"));
    assert!(!raw.contains("checkpointSeq"));
    drop(db);
    dispose(store);
}
#[test]
fn only_completed_predecessors_in_same_task_are_replaced() {
    let store = store();
    let binding = json!({"taskId":"task","agentId":"coordinator"});
    for (turn, status) in [
        ("old", "completed"),
        ("failed", "failed"),
        ("new", "completed"),
    ] {
        history::begin(
            &store,
            "p",
            turn,
            &json!({"production":{"projectId":"p","instruction":"work"},"refs":[]}),
            "m",
            &json!({"turnId":turn}),
        )
        .unwrap();
        session::start(&store, "p", turn, binding.clone(), &[Message::user(turn)]).unwrap();
        history::finish(&store, "p", turn, "answer", status, None).unwrap();
        emit(&store, turn, "turn/end", json!({"status":status}));
    }
    assert!(
        session::restore(&store, "p", "old", &binding)
            .unwrap()
            .is_none()
    );
    assert!(
        session::restore(&store, "p", "failed", &binding)
            .unwrap()
            .is_some()
    );
    assert!(
        session::restore(&store, "p", "new", &binding)
            .unwrap()
            .is_some()
    );
    assert_eq!(history::read(&store, "p").unwrap().len(), 6);
    dispose(store);
}
