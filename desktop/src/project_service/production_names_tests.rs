use super::*;

#[test]
fn naming_large_completed_media_uses_metadata_and_keeps_current_task_title() {
    let (store, mut document, _) = crate::project_service::tests::fixture();
    let task = json!({"key":"run","title":"  Updated storyboard  ","kind":"image","inputs":[]});
    document["production"] = json!({"drafts":{"run":task}});
    crate::projects::write_document(&store, document, false).unwrap();
    // A real completion has image data both in the result and normalized
    // outputs, in addition to its original reference images. Naming must not
    // copy any of those bodies into the bounded JS heap.
    let payload = "A".repeat(16 * 1024 * 1024);
    let mut job = json!({
        "projectId":"p", "status":"COMPLETED",
        "input":{"image":[format!("data:image/png;base64,{payload}")]},
        "result":{"data":[{"b64_json":payload}]},
        "outputs":[{"kind":"image","url":format!("data:image/png;base64,{payload}")}],
        "shot":{"canvasGeneration":{"task":{"key":"run","title":"Original name"}}}
    });
    let db = store.conn().unwrap();
    assert_eq!(result_name(&db, &job).unwrap(), "Updated storyboard");
    job["shot"]["canvasGeneration"]["task"]["key"] = json!("deleted-task");
    assert_eq!(result_name(&db, &job).unwrap(), "Original name");
    job["shot"]["canvasGeneration"]["task"]["title"] = json!(" ");
    assert_eq!(result_name(&db, &job).unwrap(), "未命名");
    drop(db);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
