use super::{
    export_store, files, references,
    render::Export,
    tests::{model, setup, shot},
};
use crate::{database::Store, project_service, project_storage};
use rusqlite::params;
use serde_json::{Value, json};

fn source(store: &Store) {
    let db = store.db.lock().unwrap();
    files::save(&db, store, "p", model("bag", "Bag"), 0).unwrap();
    files::save(&db, store, "p", shot("s1", "bag"), 0).unwrap();
    files::save(&db, store, "p", shot("s2", "bag"), 0).unwrap();
}
fn video(store: &Store, name: &str, seconds: f64) -> Export {
    let work = store
        .media_root()
        .join("exports")
        .join(format!("previs-{name}"));
    std::fs::create_dir_all(&work).unwrap();
    project_storage::track_file(store, "p", &work).unwrap();
    let path = work.join("previs.mp4");
    mstudio::media::run(
        mstudio::media::command("ffmpeg")
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("color=c=blue:s=64x64:r=24:d={seconds}"),
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&path),
    )
    .unwrap();
    let mut asset = mstudio::media::import(&path, &work.join("media")).unwrap();
    asset.name = "Shot 04 | 7.3–10.0s · 预演".into();
    Export {
        id: name.into(),
        file_id: "s1".into(),
        file_revision: 1,
        model_revisions: Default::default(),
        signature: name.into(),
        asset,
        saved_asset_id: None,
    }
}
fn rows(store: &Store) -> Vec<Export> {
    store
        .db
        .lock()
        .unwrap()
        .prepare("SELECT data FROM previs_exports WHERE project_id='p'")
        .unwrap()
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .map(|raw| serde_json::from_str(&raw.unwrap()).unwrap())
        .collect()
}
fn document(store: &Store) -> Value {
    project_service::load(&store.db.lock().unwrap(), "p").unwrap()
}
fn edit(store: &Store, update: impl FnOnce(&mut Value)) {
    let db = store.db.lock().unwrap();
    let before = project_service::load(&db, "p").unwrap();
    let mut after = before.clone();
    update(&mut after);
    project_service::persist(&db, &before, &after).unwrap();
}

#[test]
fn renders_stay_private_and_retire_unreferenced_previous_files() {
    let store = setup();
    source(&store);
    let first = export_store::replace(&store, "p", video(&store, "one", 0.5)).unwrap();
    let path = first.asset.path.clone();
    let next = export_store::replace(&store, "p", video(&store, "two", 1.)).unwrap();
    assert_eq!(next.id, first.id);
    assert_eq!(rows(&store).len(), 1);
    assert!(!std::path::Path::new(&path).exists());
    assert!(std::path::Path::new(&next.asset.path).exists());
    assert!(document(&store)["assets"].as_array().unwrap().is_empty());
    assert!(store.assets().unwrap().is_empty());
    assert_eq!(
        document(&store)["previsAssets"].as_array().unwrap().len(),
        1
    );
    let _ = std::fs::remove_dir_all(store.root);
}
#[test]
fn rerender_updates_live_refs_but_preserves_task_inputs_and_trimmed_references() {
    let store = setup();
    source(&store);
    let first = export_store::replace(&store, "p", video(&store, "first", 0.5)).unwrap();
    references::attach(&store, "p", "s1", &first.id, &first.asset.id).unwrap();
    let old_id = first.asset.id.clone();
    edit(&store, |doc| {
        doc["nodes"][0]["references"] = json!([{"assetId":"photo","purpose":"Keep product"},{"assetId":old_id,"purpose":"Camera","start":0.,"end":0.5}]);
        doc["nodes"][1]["references"] =
            json!([{"assetId":old_id,"purpose":"Trim","start":0.1,"end":0.3}]);
        doc["production"]["drafts"]["submitted"] =
            json!({"status":"IN_PROGRESS","inputs":[{"assetId":old_id,"role":"video-reference"}]});
    });
    let next = export_store::replace(&store, "p", video(&store, "next", 1.)).unwrap();
    let doc = document(&store);
    assert_eq!(doc["nodes"][0]["references"][0]["assetId"], "photo");
    assert_eq!(
        doc["nodes"][0]["references"][1],
        json!({"assetId":next.asset.id,"purpose":"Camera","start":0.,"end":1.})
    );
    assert_eq!(
        doc["nodes"][1]["references"][0],
        json!({"assetId":next.asset.id,"purpose":"Trim","start":0.1,"end":0.3})
    );
    assert_eq!(
        doc["production"]["drafts"]["submitted"]["inputs"][0]["assetId"],
        old_id
    );
    assert!(super::media::find(&doc, &old_id).is_some());
    assert!(std::path::Path::new(&first.asset.path).exists());
    assert!(doc["assets"].as_array().unwrap().is_empty());
    let owned: i64 = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT count(*) FROM project_assets WHERE project_id='p' AND asset_id IN (?1,?2)",
            params![old_id, next.asset.id],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(owned, 0);
    let _ = std::fs::remove_dir_all(store.root);
}
#[test]
fn explicit_collection_is_an_independent_copy_and_does_not_follow_rerenders() {
    let store = setup();
    source(&store);
    let first = export_store::replace(&store, "p", video(&store, "saved-first", 0.5)).unwrap();
    references::attach(&store, "p", "s1", &first.id, &first.asset.id).unwrap();
    let saved = references::collect(&store, "p", &first.id, &first.asset.id).unwrap();
    assert_ne!(saved["id"], first.asset.id);
    assert_ne!(saved["path"], first.asset.path);
    assert_eq!(saved["inLibrary"], true);
    assert_eq!(saved["generated"], true);
    assert_eq!(
        references::collect(&store, "p", &first.id, &first.asset.id).unwrap(),
        saved
    );
    let next = export_store::replace(&store, "p", video(&store, "saved-next", 1.)).unwrap();
    let doc = document(&store);
    assert_eq!(doc["assets"], json!([saved]));
    assert_eq!(doc["nodes"][0]["references"][0]["assetId"], next.asset.id);
    assert!(next.saved_asset_id.is_none());
    assert!(std::path::Path::new(saved["path"].as_str().unwrap()).exists());
    assert!(references::collect(&store, "p", &next.id, &first.asset.id).is_err());
    let _ = std::fs::remove_dir_all(store.root);
}
#[test]
fn stale_targets_and_failed_renders_preserve_the_existing_reference_and_video() {
    let store = setup();
    source(&store);
    let first = export_store::replace(&store, "p", video(&store, "valid", 0.5)).unwrap();
    references::attach(&store, "p", "s1", &first.id, &first.asset.id).unwrap();
    let before = document(&store);
    assert!(references::attach(&store, "p", "deleted", &first.id, &first.asset.id).is_err());
    assert!(references::attach(&store, "p", "s1", &first.id, "stale").is_err());
    assert!(references::attach(&store, "other-project", "s1", &first.id, &first.asset.id).is_err());
    let mut stale = video(&store, "stale", 1.);
    stale.file_revision = 0;
    assert!(export_store::replace(&store, "p", stale).is_err());
    let mut invalid = video(&store, "missing", 1.);
    invalid.asset.path = "/missing.mp4".into();
    assert!(export_store::replace(&store, "p", invalid).is_err());
    assert_eq!(document(&store), before);
    assert_eq!(rows(&store)[0].signature, "valid");
    assert!(std::path::Path::new(&first.asset.path).exists());
    let _ = std::fs::remove_dir_all(store.root);
}
#[test]
fn private_metadata_is_derived_and_cannot_be_written_through_project_autosave() {
    let store = setup();
    source(&store);
    let export = export_store::replace(&store, "p", video(&store, "private", 0.5)).unwrap();
    let base = document(&store);
    let mut changed = base.clone();
    changed["previsAssets"][0]["path"] = json!("/untrusted.mp4");
    let saved = crate::projects::save_merged(&store, changed, base).unwrap();
    assert_eq!(saved["previsAssets"][0]["path"], export.asset.path);
    let raw: String = store
        .db
        .lock()
        .unwrap()
        .query_row("SELECT document FROM projects WHERE id='p'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(
        serde_json::from_str::<Value>(&raw)
            .unwrap()
            .get("previsAssets")
            .is_none()
    );
    let _ = std::fs::remove_dir_all(store.root);
}
#[test]
fn project_deletion_removes_private_media_but_keeps_an_explicit_global_save() {
    let store = setup();
    source(&store);
    let export = export_store::replace(&store, "p", video(&store, "delete-private", 0.5)).unwrap();
    references::attach(&store, "p", "s1", &export.id, &export.asset.id).unwrap();
    let saved = references::collect(&store, "p", &export.id, &export.asset.id).unwrap();
    crate::asset_library::promote(&store.db.lock().unwrap(), saved["id"].as_str().unwrap())
        .unwrap();
    crate::project_storage::remove(&store, "p").unwrap();
    assert!(!std::path::Path::new(&export.asset.path).exists());
    assert!(std::path::Path::new(saved["path"].as_str().unwrap()).exists());
    let count: i64 = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT count(*) FROM previs_media WHERE project_id='p'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
    let _ = std::fs::remove_dir_all(store.root);
}
