use super::{
    messages,
    tests::{cleanup, setup},
};
use serde_json::json;

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
