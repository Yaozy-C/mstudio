use super::{
    messages,
    tests::{cleanup, setup},
};
use rusqlite::Connection;
use serde_json::json;

#[test]
fn old_history_migrates_without_losing_text_and_new_columns_are_idempotent() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE previs_messages(id INTEGER PRIMARY KEY,project_id TEXT,file_id TEXT,role TEXT,text TEXT);
        INSERT INTO previs_messages VALUES(1,'p','shot','user','Old request');").unwrap();
    messages::init(&db).unwrap();
    messages::init(&db).unwrap();
    let history = messages::list(&db, "p").unwrap();
    assert_eq!(history[0]["text"], "Old request");
    assert_eq!(history[0]["images"], json!([]));
    assert!(history[0]["requestId"].is_null());
}

#[test]
fn accepted_request_keeps_images_even_without_a_successful_model_reply() {
    let store = setup();
    let refs = json!([{"id":"image","name":"Bag.png","path":"/assets/bag.png","preview":"/previews/bag.png"}]);
    messages::save(
        &store.db.lock().unwrap(),
        "p",
        "s1",
        "request",
        "user",
        "生成预演",
        &refs,
    )
    .unwrap();
    // Simulate a model failure or app restart before an assistant response is saved.
    let root = store.root.clone();
    drop(store);
    let reopened = crate::database::Store::open(root).unwrap();
    let db = reopened.db.lock().unwrap();
    let history = messages::list(&db, "p").unwrap();
    assert_eq!(history.len(), 1);
    assert_eq!(history[0]["images"], refs);
    assert_eq!(history[0]["text"], "生成预演");
    assert_eq!(history[0]["requestId"], "request");
    assert!(!history[0].to_string().contains("base64"));
    assert!(messages::list(&db, "other-project").unwrap().is_empty());
    messages::save(&db, "p", "s1", "request", "assistant", "完成", &json!([])).unwrap();
    assert_eq!(messages::list(&db, "p").unwrap()[1]["role"], "assistant");
    drop(db);
    cleanup(reopened);
}
