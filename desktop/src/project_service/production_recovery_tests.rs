use super::{production_recovery::*, tests::fixture, *};

#[test]
fn legacy_unknown_tasks_recover_as_failed_with_or_without_a_job() {
    for reserved in [false, true] {
        let (store, mut document, _) = fixture();
        let issue = crate::app_error::AppError::new(
            "SUBMISSION_UNKNOWN",
            "submit",
            "size 须为 WIDTHxHEIGHT",
        );
        let task = json!({"key":"run","title":"Portrait","kind":"image","mode":"multi","prompt":"Keep","inputs":[],"jobId":"job","submissionId":"job","status":"UNKNOWN","error":issue.to_string(),"trackingPaused":true});
        document["production"] = json!({"drafts":{"run":task}});
        crate::projects::write_document(&store, document, false).unwrap();
        if reserved {
            let job = json!({"id":"job","projectId":"p","status":"UNKNOWN","error":issue.to_string(),"shot":{"canvasGeneration":{"task":task,"x":0,"y":0}}});
            store
                .conn()
                .unwrap()
                .execute("INSERT INTO jobs VALUES('job','p',?1)", [job.to_string()])
                .unwrap();
        }
        crate::job_recovery::recover_interrupted_submissions(&store.conn().unwrap()).unwrap();
        let report = recover(&store).unwrap();
        assert!(report.deferred.is_empty());
        let document = load(&store.conn().unwrap(), "p").unwrap();
        let task = &document["production"]["drafts"]["run"];
        assert_eq!(task["status"], "FAILED");
        assert_eq!(task["prompt"], "Keep");
        assert_ne!(task["trackingPaused"], true);
        let issue: crate::app_error::AppError =
            serde_json::from_str(task["error"].as_str().unwrap()).unwrap();
        assert_eq!(issue.code, "GENERATION_FAILED");
        assert_eq!(issue.details, "size 须为 WIDTHxHEIGHT");
        let root = store.root.clone();
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn stale_recovery_never_overwrites_concurrent_edits_or_job_changes() {
    let (store, mut doc, _) = fixture();
    let before = snapshot(&store.conn().unwrap(), "p").unwrap().unwrap();
    doc["name"] = json!("Human edit");
    crate::projects::write_document(&store, doc, false).unwrap();
    assert_eq!(
        commit(&store, "p", &before, &before.document, &[]).unwrap(),
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
        commit(&store, "p", &before, &before.document, &[]).unwrap(),
        None
    );
    let before = snapshot(&store.conn().unwrap(), "p").unwrap().unwrap();
    store
        .conn()
        .unwrap()
        .execute("UPDATE jobs SET data='{\"status\":\"UNKNOWN\"}'", [])
        .unwrap();
    assert_eq!(
        commit(&store, "p", &before, &before.document, &[]).unwrap(),
        None
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_failed_restart_reconciliation_retains_pause_and_only_a_success_clears_it() {
    let (store, mut doc, _) = fixture();
    let task = json!({"key":"run","title":"Portrait","kind":"image","mode":"multi","prompt":"Keep","inputs":[],"jobId":"job","submissionId":"job","status":"IN_QUEUE","trackingPaused":true});
    doc["production"] = json!({"drafts":{"run":task}});
    crate::projects::write_document(&store, doc, false).unwrap();
    let mut job = json!({"id":"job","projectId":"p","status":"COMPLETED","outputCount":1,
        "assets":[{"id":"result","kind":"video","name":"Wrong kind","path":"/synthetic","preview":"/synthetic.png","duration":1,"width":100,"height":100,"hasAudio":false}],
        "sync":{"stage":"reconcile","attempt":4,"paused":true,"error":"previous failure"},
        "reconciliation":{"error":"previous reconciliation failure"},
        "shot":{"canvasGeneration":{"task":task,"x":0,"y":0}}});
    store
        .conn()
        .unwrap()
        .execute("INSERT INTO jobs VALUES('job','p',?1)", [job.to_string()])
        .unwrap();
    let report = recover(&store).unwrap();
    assert_eq!(report.deferred.len(), 1);
    let document = load(&store.conn().unwrap(), "p").unwrap();
    assert_eq!(
        document["production"]["drafts"]["run"]["trackingPaused"],
        true
    );
    let saved = crate::jobs::metadata(&store, "job").unwrap();
    assert_eq!(saved["sync"]["attempt"], 4);
    assert_eq!(saved["sync"]["paused"], true);
    job["assets"][0]["kind"] = json!("image");
    store
        .conn()
        .unwrap()
        .execute("UPDATE jobs SET data=?1 WHERE id='job'", [job.to_string()])
        .unwrap();
    let report = recover(&store).unwrap();
    assert!(report.deferred.is_empty());
    let document = load(&store.conn().unwrap(), "p").unwrap();
    assert_eq!(
        document["production"]["drafts"]["run"]["trackingPaused"],
        false
    );
    assert_eq!(
        document["production"]["drafts"]["run"]["status"],
        "COMPLETED"
    );
    let saved = crate::jobs::metadata(&store, "job").unwrap();
    assert!(saved.get("sync").is_none());
    assert!(saved.get("reconciliation").is_none());
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
