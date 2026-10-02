use super::*;
#[test]
fn committed_cleanup_resumes_after_restart() {
    let f = Fixture::new();
    let path = f.file("exports/pending.mp4");
    f.store
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO pending_file_deletions VALUES('gone',?1)",
            [path.to_string_lossy()],
        )
        .unwrap();
    let reopened = Store::open(f.store.root.clone()).unwrap();
    assert!(!path.exists());
    assert_eq!(f.count("pending_file_deletions"), 0);
    drop(reopened);
}

#[tokio::test]
async fn late_saves_and_jobs_cannot_resurrect_a_deleted_project() {
    let f = Fixture::new();
    let document = json!({"id":"one","name":"one"});
    let job = json!({"id":"job","projectId":"one"});
    jobs::save(&f.store, &job).unwrap();
    remove(&f.store, "one").unwrap();
    assert!(write_document(&f.store, document, false).is_err());
    assert!(jobs::save(&f.store, &job).is_err());
    assert!(jobs::reserve(&f.store, &job).is_err());
    assert!(working(&f.store, "one").await.is_err());
    assert_eq!(f.count("jobs"), 0);
    assert_eq!(f.count("projects"), 1);
}

#[tokio::test]
async fn deletion_waits_for_in_flight_file_creation() {
    let f = Fixture::new();
    let guard = working(&f.store, "one").await.unwrap();
    assert!(f.store.files.try_write().is_err());
    let asset = f.asset("in-flight");
    drop(guard);
    let _deleting = f.store.files.write().await;
    remove(&f.store, "one").unwrap();
    assert!(!Path::new(&asset.path).exists());
}

#[test]
fn removing_from_library_does_not_lose_cleanup_ownership() {
    let f = Fixture::new();
    let asset = f.asset("removed");
    f.store
        .db
        .lock()
        .unwrap()
        .execute("DELETE FROM project_assets", [])
        .unwrap();
    f.store
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE projects SET document=?1 WHERE id='one'",
            [json!({"id":"one","assets":[asset]}).to_string()],
        )
        .unwrap();
    write_document(
        &f.store,
        json!({"id":"one","name":"one","assets":[]}),
        false,
    )
    .unwrap();
    remove(&f.store, "one").unwrap();
    assert!(!Path::new(&asset.path).exists());
}

#[cfg(unix)]
#[test]
fn file_failure_is_reported_and_retry_cleans_the_committed_queue() {
    use std::os::unix::fs::PermissionsExt;
    let f = Fixture::new();
    let path = f.file("exports/locked.mp4");
    track_file(&f.store, "one", &path).unwrap();
    let directory = path.parent().unwrap();
    std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o500)).unwrap();
    let result = remove(&f.store, "one");
    std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700)).unwrap();
    assert!(result.is_err());
    assert!(path.exists());
    assert_eq!(f.count("pending_file_deletions"), 1);
    assert_eq!(f.count("projects"), 1);
    remove(&f.store, "one").unwrap();
    assert!(!path.exists());
    assert_eq!(f.count("pending_file_deletions"), 0);
}

#[test]
fn codex_work_files_follow_project_ownership_including_existing_jobs() {
    let f = Fixture::new();
    let root = f.store.root.join("codex-images");
    let old = f.file("codex-images/old-request/state.json");
    f.file("codex-images/old-request/reference-0.png");
    let other = f.file("codex-images/other-request/state.json");
    let new = f.file("codex-images/submission/new-request/state.json");
    track_file(&f.store, "one", &root.join("submission")).unwrap();
    for (id, owner, request) in [
        ("old", "one", "old-request"),
        ("other", "two", "other-request"),
    ] {
        jobs::save(&f.store, &json!({"id":id,"projectId":owner,"providerId":"codex-image","requestId":request,"providerConfig":{"codexRoot":root}})).unwrap();
    }
    remove(&f.store, "one").unwrap();
    assert!(!old.parent().unwrap().exists());
    assert!(!new.parent().unwrap().parent().unwrap().exists());
    assert!(other.exists());
    assert!(root.exists());
}
