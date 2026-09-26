use super::*;
use crate::{jobs, projects::write_document};
use serde_json::json;
use std::path::PathBuf;

struct Fixture {
    store: Store,
}
impl Fixture {
    fn new() -> Self {
        let root = std::env::temp_dir().join(format!("mstudio-delete-{}", mstudio::media::id()));
        let store = Store::open(root).unwrap();
        for id in ["one", "two"] {
            write_document(
                &store,
                json!({"id":id,"name":id,"assets":[],"nodes":[]}),
                true,
            )
            .unwrap();
        }
        Self { store }
    }
    fn file(&self, relative: &str) -> PathBuf {
        let path = self.store.root.join(relative);
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, b"fixture").unwrap();
        path
    }
    fn asset(&self, id: &str) -> Asset {
        let asset = Asset {
            missing: false,
            generated: false,
            id: id.into(),
            name: id.into(),
            kind: "video".into(),
            path: self
                .file(&format!("assets/{id}.mp4"))
                .to_string_lossy()
                .into(),
            preview: self
                .file(&format!("previews/{id}.jpg"))
                .to_string_lossy()
                .into(),
            duration: 5.,
            width: 10,
            height: 10,
            has_audio: true,
        };
        save_asset(&self.store, "one", &asset).unwrap();
        asset
    }
    fn count(&self, table: &str) -> i64 {
        self.store
            .db
            .lock()
            .unwrap()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
            .unwrap()
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.store.root);
    }
}

#[test]
fn global_library_survives_source_project_deletion_and_can_be_reused() {
    let f = Fixture::new();
    let asset = f.asset("global");
    crate::asset_library::promote(&f.store.db.lock().unwrap(), &asset.id).unwrap();
    crate::asset_library::promote(&f.store.db.lock().unwrap(), &asset.id).unwrap();
    remove(&f.store, "one").unwrap();
    assert!(Path::new(&asset.path).exists());
    let db = f.store.db.lock().unwrap();
    assert_eq!(crate::asset_library::list(&db).unwrap().len(), 1);
    crate::asset_library::attach(&db, "two", &asset.id).unwrap();
    db.execute("DELETE FROM global_assets WHERE asset_id=?1", [&asset.id])
        .unwrap();
    drop(db);
    assert!(Path::new(&asset.path).exists());
    remove(&f.store, "two").unwrap();
    assert!(!Path::new(&asset.path).exists());
}

#[test]
fn directly_uploaded_global_assets_have_no_project_owner() {
    let f = Fixture::new();
    let mut asset = f.asset("original");
    asset.id = "direct-global".into();
    crate::asset_library::save_global(&f.store, &asset).unwrap();
    remove(&f.store, "one").unwrap();
    remove(&f.store, "two").unwrap();
    assert!(Path::new(&asset.path).exists());
    assert_eq!(
        crate::asset_library::list(&f.store.db.lock().unwrap()).unwrap()[0].id,
        "direct-global"
    );
}

#[test]
fn legacy_generation_is_classified_by_job_identity_without_changing_project_membership() {
    let f = Fixture::new();
    let asset = f.asset("generated");
    write_document(
        &f.store,
        json!({"id":"one","name":"one","assets":[asset],"nodes":[]}),
        false,
    )
    .unwrap();
    jobs::save(
        &f.store,
        &json!({"id":"job","projectId":"one","asset":asset}),
    )
    .unwrap();
    crate::asset_library::init(&f.store.db.lock().unwrap()).unwrap();
    let projects = f.store.projects().unwrap();
    let project = projects.iter().find(|p| p["id"] == "one").unwrap();
    assert_eq!(project["document"]["assets"][0]["generated"], true);
    assert_eq!(project["document"]["assets"][0]["id"], "generated");
    assert_eq!(f.count("global_assets"), 0);
}

#[test]
fn removes_files_and_all_project_rows_but_keeps_global_settings() {
    let f = Fixture::new();
    let asset = f.asset("exclusive");
    let proxy = f.file("proxies/exclusive-v1-20-30.mp4");
    let output = f.file("exports/film.mp4");
    let mix = f.file("proxies/audio-v1-mix.m4a");
    let work = f.file("render-work/run/tmp.mp4");
    for path in [&output, &mix, work.parent().unwrap()] {
        track_file(&f.store, "one", path).unwrap();
    }
    jobs::save(
        &f.store,
        &json!({"id":"job","projectId":"one","asset":asset}),
    )
    .unwrap();
    let db = f.store.db.lock().unwrap();
    db.execute_batch("INSERT INTO agent_messages(project_id,role,content,model,payload) VALUES('one','user','hello','model','{}');
        INSERT INTO agent_events(project_id,turn_id,kind,payload) VALUES('one','turn','test','{}');
        INSERT INTO project_memory VALUES('one','{}');
        INSERT INTO model_profiles VALUES('model','{}');
        INSERT INTO model_credentials VALUES('model','endpoint','secret');
        INSERT INTO agent_model_preferences VALUES('one','model');
        INSERT INTO settings VALUES('setting','keep');").unwrap();
    drop(db);
    remove(&f.store, "one").unwrap();
    for path in [
        PathBuf::from(asset.path),
        PathBuf::from(asset.preview),
        proxy,
        output,
        mix,
        work,
    ] {
        assert!(!path.exists(), "{}", path.display());
    }
    for table in [
        "assets",
        "jobs",
        "agent_messages",
        "agent_events",
        "project_memory",
        "agent_model_preferences",
        "project_assets",
        "project_files",
        "pending_file_deletions",
    ] {
        assert_eq!(f.count(table), 0, "{table}");
    }
    for table in [
        "projects",
        "model_profiles",
        "model_credentials",
        "settings",
    ] {
        assert_eq!(f.count(table), 1, "{table}");
    }
    remove(&f.store, "one").unwrap();
}

#[test]
fn shared_assets_live_until_last_project_is_deleted() {
    let f = Fixture::new();
    let asset = f.asset("shared");
    // Existing documents are authoritative even before ownership has been recorded by a save.
    f.store
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE projects SET document=?1 WHERE id='two'",
            [json!({"id":"two","nodes":[{"assetId":"shared"}]}).to_string()],
        )
        .unwrap();
    remove(&f.store, "one").unwrap();
    assert!(Path::new(&asset.path).exists());
    assert_eq!(f.count("assets"), 1);
    remove(&f.store, "two").unwrap();
    assert!(!Path::new(&asset.path).exists());
    assert_eq!(f.count("assets"), 0);
}

#[test]
fn same_file_under_different_asset_ids_is_preserved() {
    let f = Fixture::new();
    let mut alias = f.asset("original");
    alias.id = "alias".into();
    save_asset(&f.store, "two", &alias).unwrap();
    remove(&f.store, "one").unwrap();
    assert!(Path::new(&alias.path).exists());
    assert_eq!(f.count("assets"), 1);
    remove(&f.store, "two").unwrap();
    assert!(!Path::new(&alias.path).exists());
}

#[test]
fn external_files_symlinks_and_database_never_become_delete_targets() {
    let f = Fixture::new();
    let original = f.file("original.mp4");
    let link = f.store.root.join("assets/link.mp4");
    std::fs::create_dir_all(link.parent().unwrap()).unwrap();
    std::os::unix::fs::symlink(&original, &link).unwrap();
    let db = f.store.root.join("mstudio.sqlite3");
    for path in [
        &original,
        &link,
        &db,
        &f.store.root.join("assets/../mstudio.sqlite3"),
    ] {
        track_file(&f.store, "one", path).unwrap();
    }
    remove(&f.store, "one").unwrap();
    assert!(original.exists());
    assert!(link.exists());
    assert!(db.exists());
}

#[test]
fn failed_database_delete_rolls_back_without_touching_files() {
    let f = Fixture::new();
    let asset = f.asset("keep");
    f.store.db.lock().unwrap().execute_batch("CREATE TRIGGER stop_delete BEFORE DELETE ON projects BEGIN SELECT RAISE(ABORT, 'test failure'); END;").unwrap();
    assert!(remove(&f.store, "one").is_err());
    assert!(Path::new(&asset.path).exists());
    assert_eq!(f.count("projects"), 2);
    assert_eq!(f.count("assets"), 1);
    assert_eq!(f.count("pending_file_deletions"), 0);
}

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
