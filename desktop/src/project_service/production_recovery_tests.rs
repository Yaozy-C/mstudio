use super::{production_recovery::*, tests::fixture, *};

#[test]
fn stale_recovery_never_overwrites_concurrent_edits_or_job_changes() {
    let (store, mut doc, _) = fixture();
    let before = snapshot(&store.conn().unwrap(), "p").unwrap().unwrap();
    doc["name"] = json!("Human edit");
    crate::projects::write_document(&store, doc, false).unwrap();
    assert_eq!(
        commit(&store, "p", &before, &before.document).unwrap(),
        None
    );
    assert_eq!(
        load(&store.conn().unwrap(), "p").unwrap()["name"],
        "Human edit"
    );
    let before = snapshot(&store.conn().unwrap(), "p").unwrap().unwrap();
    store
        .conn()
        .unwrap()
        .execute("INSERT INTO jobs VALUES('reserved','p','{}')", [])
        .unwrap();
    assert_eq!(
        commit(&store, "p", &before, &before.document).unwrap(),
        None
    );
    let before = snapshot(&store.conn().unwrap(), "p").unwrap().unwrap();
    store
        .conn()
        .unwrap()
        .execute("UPDATE jobs SET data='{\"status\":\"UNKNOWN\"}'", [])
        .unwrap();
    assert_eq!(
        commit(&store, "p", &before, &before.document).unwrap(),
        None
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn startup_recovers_status_despite_corrupt_omitted_input_and_is_idempotent() {
    let (store, mut doc, _) = fixture();
    let task = json!({"key":"run","kind":"image","mode":"multi","prompt":"Keep","inputs":[],"jobId":"job","submissionId":"job","status":"SUBMITTING"});
    doc["production"] = json!({"drafts":{"run":task}});
    crate::projects::write_document(&store, doc, false).unwrap();
    let mut job = json!({"id":"job","projectId":"p","status":"FAILED","error":"Synthetic failure","input":{"image":"x".repeat(10000)},"shot":{"canvasGeneration":{"task":task,"x":0,"y":0}}});
    {
        let db = store.conn().unwrap();
        db.execute("INSERT INTO jobs VALUES('job','p','{}')", [])
            .unwrap();
        crate::database::blobs::pack(&db, crate::database::blobs::Owner::Job("job"), &mut job)
            .unwrap();
        db.execute("UPDATE jobs SET data=?1", [job.to_string()])
            .unwrap();
        db.execute("UPDATE content_blobs SET data=x'0000'", [])
            .unwrap();
        assert!(
            crate::database::blobs::hydrate(&db, crate::database::blobs::Owner::Job("job"), job)
                .is_err()
        );
    }
    assert_eq!(recover(&store).unwrap(), vec!["p"]);
    let saved = load(&store.conn().unwrap(), "p").unwrap();
    assert_eq!(saved["production"]["drafts"]["run"]["status"], "FAILED");
    assert_eq!(saved["revision"], 0); // Runtime status does not change creative content.
    assert!(recover(&store).unwrap().is_empty());
    assert_eq!(load(&store.conn().unwrap(), "p").unwrap(), saved);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
