use super::*;
#[test]
fn independent_pools_skip_slow_jobs_and_resume_durable_backoff() {
    let jobs: Vec<_> = (0..100)
        .map(|i| json!({"id":i.to_string(),"status":"IN_PROGRESS","requestId":"r"}))
        .collect();
    let mut active = HashMap::from([("0".into(), Work::Status)]);
    let next = candidates(jobs, &active, 100);
    assert_eq!(next.len(), 3);
    assert!(next.iter().all(|(id, _)| id != "0"));
    for (id, stage) in next {
        active.insert(id, stage);
    }
    let downloads = (0..100)
        .map(|i| json!({"id":format!("d{i}"),"status":"COMPLETED","outputCount":1}))
        .collect();
    assert_eq!(candidates(downloads, &active, 100).len(), 2);
    assert!(candidates(vec![json!({"id":"paused","status":"IN_PROGRESS","requestId":"r","sync":{"paused":true}}),
        json!({"id":"later","status":"IN_PROGRESS","requestId":"r","sync":{"nextAt":101}})], &HashMap::new(), 100).is_empty());
}
#[test]
fn retry_backoff_and_pause_survive_serialization_and_success_resets_them() {
    let mut job = json!({"id":"job","status":"IN_PROGRESS","requestId":"r"});
    for attempt in 1..=4 {
        update_sync(
            &mut job,
            Work::Status,
            Some(anyhow::anyhow!("temporary network failure")),
            100,
        );
        job = serde_json::from_str(&job.to_string()).unwrap();
        assert_eq!(job["sync"]["attempt"], attempt);
        assert_eq!(job["sync"]["paused"], attempt == 4);
        assert!(candidates(vec![job.clone()], &HashMap::new(), 100).is_empty());
    }
    update_sync(&mut job, Work::Status, None, 200);
    assert_eq!(job["sync"]["attempt"], 0);
    assert_eq!(job["sync"]["nextAt"], 204);
    assert_eq!(candidates(vec![job], &HashMap::new(), 204).len(), 1);
}
#[test]
fn partial_imports_resume_at_missing_output_and_unknown_submissions_are_never_replayed() {
    let job = json!({"status":"COMPLETED","outputCount":3,"assets":[{"id":"a"},null,{"id":"c"}]});
    assert_eq!(work(&job), Some(Work::Import(1)));
    assert_eq!(
        work(&json!({"status":"COMPLETED","outputCount":1,"asset":{"id":"a"}})),
        None
    );
    assert_eq!(work(&json!({"status":"UNKNOWN"})), None);
    assert_eq!(work(&json!({"status":"FAILED","outputCount":0})), None);
    assert_eq!(
        work(&json!({"status":"COMPLETED","outputCount":0})),
        Some(Work::ValidateOutputs)
    );
}

#[test]
fn reconciliation_uses_local_work_with_the_existing_bounded_backoff() {
    let mut job = json!({"id":"job","status":"COMPLETED","outputCount":1,"asset":{"id":"done"},
        "reconciliation":{"error":"project update pending"}});
    assert_eq!(work(&job), Some(Work::Reconcile));
    for attempt in 1..=4 {
        update_sync(
            &mut job,
            Work::Reconcile,
            Some(jobs::reconciliation_issue(&anyhow::anyhow!("project update pending")).into()),
            100,
        );
        assert_eq!(job["sync"]["stage"], "reconcile");
        assert_eq!(job["sync"]["attempt"], attempt);
        assert_eq!(job["sync"]["paused"], attempt == 4);
        assert!(candidates(vec![job.clone()], &HashMap::new(), 100).is_empty());
    }
    job.as_object_mut().unwrap().remove("reconciliation");
    update_sync(&mut job, Work::Reconcile, None, 200);
    assert_eq!(work(&job), None);
}

#[test]
fn stale_status_failure_preserves_another_stages_backoff() {
    let mut job = json!({"status":"COMPLETED","outputCount":1,"sync":{"stage":"import","attempt":2,"error":"keep this import failure","paused":false,"nextAt":200}});
    let before = job.clone();
    assert!(!update_sync(
        &mut job,
        Work::Status,
        Some(anyhow::anyhow!("late status failure")),
        100
    ));
    assert_eq!(job, before);
    assert!(!update_sync(&mut job, Work::Status, None, 100));
    assert_eq!(job, before);
}

#[test]
fn malformed_completed_count_is_a_non_retryable_local_failure() {
    for value in [json!(null), json!(0), json!(-1), json!(1.5)] {
        let mut job = json!({"id":"job","status":"COMPLETED","outputCount":value});
        assert_eq!(work(&job), Some(Work::ValidateOutputs));
        let error = jobs::outputs::count(&job).unwrap_err();
        assert!(update_sync(
            &mut job,
            Work::ValidateOutputs,
            Some(error),
            100
        ));
        assert_eq!(job["sync"]["paused"], true);
    }
}
#[test]
fn tracking_write_failure_is_returned_and_cannot_count_as_recorded_backoff() {
    let (store, _, _) = crate::project_service::tests::fixture();
    store
        .conn()
        .unwrap()
        .execute(
            "INSERT INTO jobs VALUES('job','p',?1)",
            [
                json!({"id":"job","projectId":"p","status":"IN_PROGRESS","requestId":"remote"})
                    .to_string(),
            ],
        )
        .unwrap();
    store
        .conn()
        .unwrap()
        .execute_batch("PRAGMA query_only=ON")
        .unwrap();
    let error = record_saved(
        &store,
        "job",
        Work::Status,
        Some(anyhow::anyhow!("provider failure")),
    )
    .unwrap_err();
    assert!(error.to_string().contains("readonly"));
    assert!(jobs::metadata(&store, "job").unwrap()["sync"].is_null());
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
