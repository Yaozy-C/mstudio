use super::*;
use crate::{asset_library, project_storage, projects::write_document};
use mstudio::model::Asset;
use serde_json::json;
use std::fs;

struct Fixture {
    base: PathBuf,
    store: Store,
    asset: Asset,
}
impl Fixture {
    fn new() -> Self {
        let base =
            std::env::temp_dir().join(format!("mstudio-storage-test-{}", mstudio::media::id()));
        let store = Store::open(base.join("app")).unwrap();
        fs::create_dir_all(store.root.join("assets")).unwrap();
        let path = store.root.join("assets/reference.txt");
        fs::write(&path, "original content").unwrap();
        let asset: Asset = serde_json::from_value(json!({"id":"ref","name":"reference.txt","kind":"text","path":path,"preview":"","duration":0,"width":0,"height":0,"hasAudio":false})).unwrap();
        write_document(&store, json!({"id":"p","name":"project","assets":[asset],"nodes":[{"id":"shot","assetId":"ref"}],"clips":[{"id":"clip","assetId":"ref"}]}), true).unwrap();
        // The document remembers ownership; insert the canonical file record separately.
        store
            .db
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO assets VALUES('ref',?1)",
                [serde_json::to_string(&asset).unwrap()],
            )
            .unwrap();
        Self { base, store, asset }
    }
    fn target(&self) -> PathBuf {
        self.base.join("Mstudio")
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.base);
    }
}

#[test]
fn migration_updates_all_references_and_global_membership_without_moving_database() {
    let f = Fixture::new();
    asset_library::promote(&f.store.db.lock().unwrap(), "ref").unwrap();
    crate::jobs::save(
        &f.store,
        &json!({"id":"job","projectId":"p","asset":f.asset}),
    )
    .unwrap();
    project_storage::track_file(&f.store, "p", Path::new(&f.asset.path)).unwrap();
    let result = migration::migrate(&f.store, &f.target(), |_| Ok(())).unwrap();
    assert_eq!(result["files"], 1);
    assert!(!Path::new(&f.asset.path).exists());
    assert!(f.store.root.join("mstudio.sqlite3").exists());
    let expected = f
        .target()
        .parent()
        .unwrap()
        .canonicalize()
        .unwrap()
        .join("Mstudio/assets/reference.txt");
    assert_eq!(fs::read_to_string(&expected).unwrap(), "original content");
    let p = &f.store.projects().unwrap()[0]["document"];
    assert_eq!(p["assets"][0]["path"], expected.to_string_lossy().as_ref());
    assert_eq!(p["nodes"][0]["assetId"], "ref");
    assert_eq!(p["clips"][0]["assetId"], "ref");
    let global = asset_library::list(&f.store.db.lock().unwrap()).unwrap();
    assert_eq!(global[0].path, expected.to_string_lossy());
    assert!(!global[0].missing);
    let reopened = Store::open(f.store.root.clone()).unwrap();
    assert_eq!(
        reopened.media_root(),
        expected.parent().unwrap().parent().unwrap()
    );
    project_storage::remove(&reopened, "p").unwrap();
    assert!(expected.exists()); // Global ownership still protects the migrated file.
}

#[test]
fn missing_files_remain_placeholders_and_restore_without_losing_references() {
    let f = Fixture::new();
    fs::remove_file(&f.asset.path).unwrap();
    assert!(statuses(&f.store, &["ref".into()]).unwrap()["ref"]);
    migration::migrate(&f.store, &f.target(), |_| Ok(())).unwrap();
    let projects = f.store.projects().unwrap();
    let p = &projects[0]["document"];
    assert_eq!(p["assets"].as_array().unwrap().len(), 1);
    assert_eq!(p["assets"][0]["missing"], true);
    assert_eq!(p["nodes"][0]["assetId"], "ref");
    let path = Path::new(p["assets"][0]["path"].as_str().unwrap());
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, "restored").unwrap();
    assert!(!statuses(&f.store, &["ref".into()]).unwrap()["ref"]);
    assert_eq!(
        f.store.projects().unwrap()[0]["document"]["assets"][0]["missing"],
        false
    );
}

#[test]
fn failed_migration_keeps_original_files_and_records() {
    let f = Fixture::new();
    f.store.db.lock().unwrap().execute_batch("CREATE TRIGGER fail_migration BEFORE UPDATE ON assets BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
    assert!(migration::migrate(&f.store, &f.target(), |_| Ok(())).is_err());
    assert_eq!(f.store.media_root(), f.store.root);
    assert_eq!(
        fs::read_to_string(&f.asset.path).unwrap(),
        "original content"
    );
    assert!(!f.target().exists());
    assert_eq!(
        f.store.projects().unwrap()[0]["document"]["assets"][0]["path"],
        f.asset.path
    );
}

#[test]
fn existing_destinations_and_nested_paths_are_not_overwritten() {
    let f = Fixture::new();
    fs::create_dir(f.target()).unwrap();
    fs::write(f.target().join("keep.txt"), "keep").unwrap();
    assert!(migration::migrate(&f.store, &f.target(), |_| Ok(())).is_err());
    assert!(migration::migrate(&f.store, &f.store.root.join("nested"), |_| Ok(())).is_err());
    assert_eq!(
        fs::read_to_string(f.target().join("keep.txt")).unwrap(),
        "keep"
    );
}

#[test]
fn late_project_save_cannot_reintroduce_old_paths_after_migration() {
    let f = Fixture::new();
    let stale = f.store.projects().unwrap()[0]["document"].clone();
    migration::migrate(&f.store, &f.target(), |_| Ok(())).unwrap();
    write_document(&f.store, stale, false).unwrap();
    let p = f.store.projects().unwrap();
    assert!(
        p[0]["document"]["assets"][0]["path"]
            .as_str()
            .unwrap()
            .starts_with(f.store.media_root().to_str().unwrap())
    );
    let mut external = json!({"path":f.store.root.join("assets-unrelated/file.txt"), "other":f.store.root.join("mstudio.sqlite3")});
    let before = external.clone();
    f.store.normalize_paths(&mut external);
    assert_eq!(external, before);
}

#[test]
fn new_imports_and_project_cleanup_use_the_new_directory() {
    let f = Fixture::new();
    migration::migrate(&f.store, &f.target(), |_| Ok(())).unwrap();
    let original = f.base.join("new.txt");
    fs::write(&original, "new upload").unwrap();
    let imported = crate::imports::import_paths(&f.store, "p", vec![original.clone()]);
    assert!(imported.errors.is_empty());
    assert_eq!(imported.assets.len(), 1);
    assert!(Path::new(&imported.assets[0].path).starts_with(f.store.media_root()));
    project_storage::remove(&f.store, "p").unwrap();
    assert!(!Path::new(&imported.assets[0].path).exists());
    assert!(!f.store.media_root().join("assets/reference.txt").exists());
    assert!(original.exists());
    assert!(f.store.root.join("mstudio.sqlite3").exists());
}
