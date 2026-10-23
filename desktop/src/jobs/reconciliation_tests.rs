use super::*;

#[test]
fn remote_submission_facts_survive_a_failed_project_write() {
    let (store, mut document, _) = crate::project_service::tests::fixture();
    let task = json!({"key":"run","title":"Portrait","kind":"image","mode":"multi","prompt":"Keep", "inputs":[],"status":"SUBMITTING","jobId":"job","submissionId":"job"});
    document["production"] = json!({"drafts":{"run":task}});
    crate::projects::write_document(&store, document, false).unwrap();
    let before = crate::project_service::load(&store.conn().unwrap(), "p").unwrap();
    let mut job = json!({"id":"job","projectId":"p","providerId":"fal","endpoint":"fal-ai/test",
        "status":"SUBMITTING","input":{"prompt":"approved input".repeat(1000)},
        "shot":{"canvasGeneration":{"task":task,"x":0,"y":0}}});
    assert!(super::super::reserve(&store, &job).unwrap());
    job["status"] = json!("IN_PROGRESS");
    job["requestId"] = json!("remote-original");
    store.conn().unwrap().execute_batch("CREATE TRIGGER reject_sync BEFORE UPDATE ON projects BEGIN SELECT RAISE(ABORT,'synthetic project write failure'); END;").unwrap();
    let error = super::super::save(&store, &job).unwrap_err();
    assert_eq!(
        error.downcast_ref::<AppError>().unwrap().code,
        "PROJECT_SYNC_FAILED"
    );
    let saved = super::super::get(&store, "job").unwrap();
    assert_eq!(saved["requestId"], "remote-original");
    assert_eq!(saved["input"], job["input"]);
    assert!(
        saved["reconciliation"]["error"]
            .as_str()
            .unwrap()
            .contains("synthetic project write failure")
    );
    assert_eq!(
        crate::project_service::load(&store.conn().unwrap(), "p").unwrap(),
        before
    );
    assert!(
        save_tracking(
            &store,
            "job",
            Some(&json!({"stage":"reconcile","attempt":3,"error":"previous error"})),
        )
        .unwrap()
        .is_some()
    );
    {
        let db = store.conn().unwrap();
        db.execute_batch("DROP TRIGGER reject_sync").unwrap();
    }
    reconcile(&store, "job").unwrap();
    assert_eq!(
        super::super::get(&store, "job").unwrap()["input"],
        job["input"]
    );
    let saved = metadata(&store, "job").unwrap();
    assert!(saved.get("reconciliation").is_none());
    assert!(saved.get("sync").is_none());
    assert_eq!(saved["requestId"], "remote-original");
    let after = crate::project_service::load(&store.conn().unwrap(), "p").unwrap();
    assert_eq!(
        after["production"]["drafts"]["run"]["status"],
        "IN_PROGRESS"
    );
    assert!(after["production"]["drafts"]["run"].get("error").is_none());
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn reserved_submission_does_not_depend_on_project_rendering() {
    let (store, _, _) = crate::project_service::tests::fixture();
    store.conn().unwrap().execute_batch("CREATE TRIGGER reject_sync BEFORE UPDATE ON projects BEGIN SELECT RAISE(ABORT,'synthetic project write failure'); END;").unwrap();
    let job = json!({"id":"reserved","projectId":"p","providerId":"fal","input":{},"endpoint":"fal-ai/test","status":"SUBMITTING"});
    assert!(super::super::reserve(&store, &job).unwrap());
    assert!(!super::super::reserve(&store, &job).unwrap());
    assert_eq!(
        metadata(&store, "reserved").unwrap()["status"],
        "SUBMITTING"
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn invalid_job_output_has_a_visible_project_diagnostic_and_a_durable_pending_job() {
    let (store, mut document, _) = crate::project_service::tests::fixture();
    let task = json!({"key":"run","title":"Portrait","kind":"image","mode":"multi","prompt":"Keep","inputs":[],"status":"IN_QUEUE","jobId":"job","submissionId":"job"});
    document["production"] = json!({"drafts":{"run":task}});
    crate::projects::write_document(&store, document, false).unwrap();
    let mut job = json!({"id":"job","projectId":"p","status":"COMPLETED","outputCount":1,
        "assets":[{"id":"output","kind":"video","name":"Wrong kind","path":"/synthetic","preview":"/synthetic.png","duration":1,"width":100,"height":100,"hasAudio":false}],
        "shot":{"canvasGeneration":{"task":task,"x":0,"y":0}}});
    assert!(super::super::save(&store, &job).is_err());
    let saved = metadata(&store, "job").unwrap();
    let project = crate::project_service::load(&store.conn().unwrap(), "p").unwrap();
    let error = project["production"]["drafts"]["run"]["error"]
        .as_str()
        .unwrap();
    let issue: AppError = serde_json::from_str(error).unwrap();
    let durable: AppError =
        serde_json::from_str(saved["reconciliation"]["error"].as_str().unwrap()).unwrap();
    assert_eq!(issue.code, "PROJECT_SYNC_FAILED");
    assert_eq!(durable.code, issue.code);
    assert_eq!(durable.stage, issue.stage);
    assert_eq!(durable.details, issue.details);
    assert_eq!(project["production"]["drafts"]["run"]["status"], "IN_QUEUE");
    assert_eq!(project["assets"], json!([]));
    save_tracking(
        &store,
        "job",
        Some(&json!({"stage":"reconcile","attempt":4,"paused":true,"error":error})),
    )
    .unwrap();
    let paused = crate::project_service::load(&store.conn().unwrap(), "p").unwrap();
    assert_eq!(
        paused["production"]["drafts"]["run"]["trackingPaused"],
        true
    );
    assert_eq!(paused["production"]["drafts"]["run"]["status"], "IN_QUEUE");
    assert_eq!(paused["assets"], json!([]));
    job["assets"][0]["kind"] = json!("image");
    super::super::save(&store, &job).unwrap();
    let project = crate::project_service::load(&store.conn().unwrap(), "p").unwrap();
    assert_eq!(
        project["production"]["drafts"]["run"]["status"],
        "COMPLETED"
    );
    assert!(
        project["production"]["drafts"]["run"]
            .get("error")
            .is_none()
    );
    assert!(
        metadata(&store, "job")
            .unwrap()
            .get("reconciliation")
            .is_none()
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
