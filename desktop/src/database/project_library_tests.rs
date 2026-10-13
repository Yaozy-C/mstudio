use super::*;

#[test]
fn summaries_preserve_order_and_skip_missing_covers_without_loading_documents() {
    let (store, mut doc, _) = crate::project_service::tests::fixture();
    let source = store.root.join("source.png");
    std::fs::write(&source, "synthetic").unwrap();
    doc["assets"] = json!([
        {"id":"missing","path":"/missing-synthetic.png","preview":"/missing.jpg"},
        {"id":"present","path":source,"preview":"/cover.jpg"}
    ]);
    crate::projects::write_document(&store, doc.clone(), false).unwrap();
    doc["id"] = json!("new");
    doc["name"] = json!("New");
    doc["assets"] = json!([]);
    crate::projects::write_document(&store, doc, true).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("UPDATE projects SET updated=updated+10 WHERE id='new'", [])
        .unwrap();
    let summaries = store.project_summaries().unwrap();
    assert_eq!(summaries[0]["id"], "new");
    assert_eq!(summaries[0]["preview"], "");
    assert_eq!(summaries[1]["preview"], "/cover.jpg");
    assert_eq!(summaries[1]["width"], 1280);
    assert_eq!(summaries[1]["clipCount"], 0);
    assert!(summaries.iter().all(|s| s.get("document").is_none()));
    let document = store.project("p").unwrap();
    assert_eq!(document["assets"][0]["missing"], true);
    assert_eq!(document["assets"][1]["missing"], false);
    assert_eq!(document["nodes"].as_array().unwrap().len(), 2);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
