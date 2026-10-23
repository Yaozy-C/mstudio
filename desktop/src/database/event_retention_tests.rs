use super::*;
use crate::assistant::{history, journal};

fn store() -> Store {
    let root = std::env::temp_dir().join(format!("mstudio-retention-{}", mstudio::media::id()));
    let store = Store::open(root).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','p','{}',0)", [])
        .unwrap();
    store
}
fn begin(store: &Store, turn: &str) {
    let request = json!({"production":{"projectId":"p","instruction":"request"},"refs":[]});
    history::begin(store, "p", turn, &request, "model", &json!({"turnId":turn})).unwrap();
}
fn count(db: &Connection, turn: &str) -> i64 {
    db.query_row(
        "SELECT count(*) FROM agent_events WHERE turn_id=?1",
        [turn],
        |r| r.get(0),
    )
    .unwrap()
}
#[test]
fn live_progress_keeps_crash_checkpoint_without_appending_fragments() {
    let store = store();
    begin(&store, "t");
    for n in 1..=50 {
        journal::append(
            &store,
            "p",
            "t",
            "assistant/partial",
            json!({"text":"x".repeat(n),"delta":"x"}),
        )
        .unwrap();
        journal::append(&store, "p", "t", "request/start", json!({})).unwrap();
    }
    assert_eq!(count(&store.db.lock().unwrap(), "t"), 0);
    assert_eq!(
        history::read(&store, "p").unwrap()[1].content,
        "x".repeat(50)
    );
    let root = store.root.clone();
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    let messages = history::read(&store, "p").unwrap();
    assert_eq!(messages[1].content, "x".repeat(50));
    assert_eq!(
        messages[1].attribution.as_ref().unwrap()["status"],
        "interrupted"
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[path = "event_retention_tests_thinking.rs"]
mod thinking;
