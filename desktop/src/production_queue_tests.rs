use super::*;

fn fixture() -> (Store, Value) {
    let (store, mut doc, _) = crate::project_service::tests::fixture();
    let task = json!({"key":"run","kind":"image","title":"Portrait","prompt":"Keep","mode":"multi","inputs":[],"submissionId":"job","status":"UPLOADING"});
    doc["production"] = json!({"drafts":{"run":task}});
    crate::projects::write_document(&store, doc, false).unwrap();
    (store, task)
}
fn cleanup(store: Store) {
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn preflight_failure_without_reservation_records_the_original_error() {
    let (store, _) = fixture();
    submission_failed(
        &store,
        "p",
        "run",
        "job",
        anyhow::anyhow!("reference upload denied"),
    )
    .unwrap();
    let doc = project_service::load(&store.conn().unwrap(), "p").unwrap();
    assert_eq!(doc["production"]["drafts"]["run"]["status"], "FAILED");
    assert!(
        doc["production"]["drafts"]["run"]["error"]
            .as_str()
            .unwrap()
            .contains("reference upload denied")
    );
    assert!(crate::jobs::metadata(&store, "job").is_err());
    cleanup(store);
}

#[test]
fn local_execution_failure_does_not_blame_model_settings_or_submit_a_job() {
    let (store, _) = fixture();
    let error = project_service::domain::execute(json!({
        "action":"inspect", "document":null, "args":{}
    }))
    .unwrap_err();
    submission_failed(&store, "p", "run", "job", error).unwrap();
    let doc = project_service::load(&store.conn().unwrap(), "p").unwrap();
    let issue: crate::app_error::AppError = serde_json::from_str(
        doc["production"]["drafts"]["run"]["error"]
            .as_str()
            .unwrap(),
    )
    .unwrap();
    assert_eq!(issue.code, "OPERATION_FAILED");
    assert_eq!(issue.stage, "prepare");
    assert!(issue.details.contains("inspect"));
    assert_ne!(issue.details, "Domain execution failed");
    assert!(crate::jobs::metadata(&store, "job").is_err());
    cleanup(store);
}
#[test]
fn failure_status_write_error_is_returned_with_both_causes() {
    let (store, _) = fixture();
    store.conn().unwrap().execute_batch("CREATE TRIGGER reject_failure BEFORE UPDATE ON projects BEGIN SELECT RAISE(ABORT,'project write denied'); END;").unwrap();
    let error = submission_failed(
        &store,
        "p",
        "run",
        "job",
        anyhow::anyhow!("reference upload denied"),
    )
    .unwrap_err();
    assert!(error.to_string().contains("project write denied"));
    assert!(error.to_string().contains("reference upload denied"));
    assert_eq!(
        project_service::load(&store.conn().unwrap(), "p").unwrap()["production"]["drafts"]["run"]
            ["status"],
        "UPLOADING"
    );
    cleanup(store);
}
#[test]
fn an_existing_remote_job_is_reconciled_without_reclassifying_or_resubmitting() {
    let (store, task) = fixture();
    let job = json!({"id":"job","projectId":"p","status":"IN_PROGRESS","requestId":"remote", "shot":{"canvasGeneration":{"task":task,"x":0,"y":0}}});
    assert!(crate::jobs::reserve(&store, &job).unwrap());
    submission_failed(
        &store,
        "p",
        "run",
        "job",
        anyhow::anyhow!("IPC response lost"),
    )
    .unwrap();
    let saved = crate::jobs::metadata(&store, "job").unwrap();
    assert_eq!(saved["requestId"], "remote");
    assert_eq!(saved["status"], "IN_PROGRESS");
    assert_eq!(
        project_service::load(&store.conn().unwrap(), "p").unwrap()["production"]["drafts"]["run"]
            ["status"],
        "IN_PROGRESS"
    );
    cleanup(store);
}

#[test]
fn provider_submission_failure_reconciles_as_a_retryable_task() {
    let (store, task) = fixture();
    let mut job = json!({"id":"job","projectId":"p","status":"SUBMITTING","shot":{"canvasGeneration":{"task":task,"x":0,"y":0}}});
    assert!(crate::jobs::reserve(&store, &job).unwrap());
    let error = crate::app_error::AppError::http(422, "fal", "size 须为 WIDTHxHEIGHT").into();
    crate::job_recovery::submission_failed(&mut job, &error);
    crate::jobs::save(&store, &job).unwrap();
    let document = project_service::load(&store.conn().unwrap(), "p").unwrap();
    let task = &document["production"]["drafts"]["run"];
    assert_eq!(task["status"], "FAILED");
    let issue: crate::app_error::AppError =
        serde_json::from_str(task["error"].as_str().unwrap()).unwrap();
    assert_eq!(issue.code, "INVALID_INPUT");
    assert_eq!(issue.details, "size 须为 WIDTHxHEIGHT");
    assert_eq!(
        crate::jobs::metadata(&store, "job").unwrap()["status"],
        "FAILED"
    );
    cleanup(store);
}
