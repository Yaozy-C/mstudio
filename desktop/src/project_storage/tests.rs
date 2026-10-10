use super::*;
use crate::{jobs, projects::write_document};
use serde_json::json;
use std::path::PathBuf;

pub(super) struct Fixture {
    pub(super) store: Store,
}
impl Fixture {
    pub(super) fn new() -> Self {
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
    pub(super) fn asset(&self, id: &str) -> Asset {
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
    pub(super) fn count(&self, table: &str) -> i64 {
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
        INSERT INTO model_profiles VALUES('model','{}');
        INSERT INTO model_credentials VALUES('model','endpoint','secret');
        INSERT INTO agent_model_preferences VALUES('one','model');
        INSERT INTO settings VALUES('setting','keep');").unwrap();
    drop(db);
    let settings_count = f.count("settings");
    remove(&f.store, "one").unwrap();
    assert_eq!(f.count("settings"), settings_count);
    assert_eq!(f.store.setting("setting").unwrap(), "keep");
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
        "agent_model_preferences",
        "project_assets",
        "project_files",
        "pending_file_deletions",
    ] {
        assert_eq!(f.count(table), 0, "{table}");
    }
    for table in ["projects", "model_profiles", "model_credentials"] {
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

#[cfg(unix)]
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

#[path = "cleanup_tests.rs"]
mod cleanup;
