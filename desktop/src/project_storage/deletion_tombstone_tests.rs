use super::*;
use serde_json::json;

#[test]
fn failed_deletion_rolls_back_the_tombstone_with_the_project_and_retries_cleanly() {
    let f = super::super::tests::Fixture::new();
    let asset = f.asset("removed");
    let before = json!({"id":"one","name":"one","assets":[asset],"nodes":[],"clips":[],"captions":[],"tracks":[]});
    crate::projects::write_document(&f.store, before, false).unwrap();
    let before = crate::project_service::load(&f.store.conn().unwrap(), "one").unwrap();
    let mut after = before.clone();
    after["removedAssetIds"] = json!([asset.id]);
    f.store.conn().unwrap().execute_batch("CREATE TRIGGER reject_delete BEFORE DELETE ON assets BEGIN SELECT RAISE(ABORT,'synthetic deletion failure'); END;").unwrap();
    assert!(crate::projects::save_merged(&f.store, after.clone(), before.clone()).is_err());
    assert!(read(&f.store.conn().unwrap(), "one").unwrap().is_empty());
    assert!(std::path::Path::new(&asset.path).exists());
    f.store
        .conn()
        .unwrap()
        .execute_batch("DROP TRIGGER reject_delete")
        .unwrap();
    crate::projects::save_merged(&f.store, after, before).unwrap();
    assert!(
        read(&f.store.conn().unwrap(), "one")
            .unwrap()
            .contains(&asset.id)
    );
    assert!(!std::path::Path::new(&asset.path).exists());
}

#[test]
fn deleted_ids_redact_an_in_flight_worker_snapshot() {
    let f = super::super::tests::Fixture::new();
    let asset = f.asset("removed");
    let before = json!({"id":"one","name":"one","assets":[asset],"nodes":[],"clips":[],"captions":[],"tracks":[]});
    crate::projects::write_document(&f.store, before, false).unwrap();
    let before = crate::project_service::load(&f.store.conn().unwrap(), "one").unwrap();
    let mut after = before.clone();
    after["removedAssetIds"] = json!([asset.id]);
    crate::projects::save_merged(&f.store, after, before).unwrap();
    let job = json!({"id":"job","projectId":"one","providerId":"fal","requestId":"remote",
        "status":"COMPLETED","asset":asset,"assets":[asset],
        "result":{"image":"data:image/png;base64,aW1hZ2U="},
        "outputs":[{"kind":"image","url":"data:image/png;base64,aW1hZ2U="}],
        "shot":{"canvasGeneration":{"task":{"key":"run","kind":"video"},"x":0,"y":0}}});
    crate::jobs::save(&f.store, &job).unwrap();
    let saved = crate::jobs::get(&f.store, "job").unwrap();
    assert_eq!(saved["requestId"], "remote");
    assert_eq!(saved["assets"][0], json!({"id":asset.id,"deleted":true}));
    assert!(saved["result"].is_null());
    assert_eq!(saved["outputs"][0], json!({"deleted":true}));
    assert_eq!(f.count("job_content"), 0);
    assert!(!std::path::Path::new(&asset.path).exists());
}
