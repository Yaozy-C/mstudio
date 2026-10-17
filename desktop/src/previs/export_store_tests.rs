use super::{
    export_store, files,
    render::Export,
    sharing,
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
        shared_asset_id: None,
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
fn successful_renders_replace_one_result_and_retire_old_private_files() {
    let store = setup();
    source(&store);
    let first = export_store::replace(&store, "p", video(&store, "one", 0.5)).unwrap();
    let original_path = first.asset.path.clone();
    let next = export_store::replace(&store, "p", video(&store, "two", 1.)).unwrap();
    assert_eq!(next.id, first.id);
    assert_eq!(rows(&store).len(), 1);
    assert_eq!(rows(&store)[0].signature, "two");
    assert!(std::path::Path::new(&next.asset.path).exists());
    assert!(!std::path::Path::new(&original_path).exists());
    assert!(next.shared_asset_id.is_none());
    let mut other = video(&store, "other", 0.5);
    other.file_id = "s2".into();
    export_store::replace(&store, "p", other).unwrap();
    assert_eq!(rows(&store).len(), 2);
    let _ = std::fs::remove_dir_all(store.root);
}

#[test]
fn shot_references_follow_the_new_video_preserving_purpose_trims_and_other_media() {
    let store = setup();
    source(&store);
    let first = export_store::replace(&store, "p", video(&store, "shared-one", 0.5)).unwrap();
    let shared = sharing::set_shared(&store, "p", &first.id, true).unwrap();
    let old_id = shared["assetId"].as_str().unwrap().to_owned();
    edit(&store, |doc| {
        doc["nodes"][0]["references"] = json!([{"assetId":"photo","purpose":"Keep product appearance"},{"assetId":old_id,"purpose":"Follow the camera","start":0,"end":0.5}]);
        doc["nodes"][1]["references"] =
            json!([{"assetId":old_id,"purpose":"Trimmed motion","start":0.1,"end":0.3}]);
        doc["clips"] = json!([{"id":"existing-clip","assetId":old_id,"trimIn":0,"trimOut":0.4}]);
    });
    let next = export_store::replace(&store, "p", video(&store, "shared-two", 1.)).unwrap();
    let new_id = next.shared_asset_id.unwrap();
    let doc = document(&store);
    assert_ne!(old_id, new_id);
    assert_eq!(doc["nodes"][0]["references"][0]["assetId"], "photo");
    assert_eq!(
        doc["nodes"][0]["references"][1],
        json!({"assetId":new_id,"purpose":"Follow the camera","start":0.,"end":1.})
    );
    assert_eq!(
        doc["nodes"][1]["references"][0],
        json!({"assetId":new_id,"purpose":"Trimmed motion","start":0.1,"end":0.3})
    );
    assert_eq!(doc["clips"][0]["assetId"], old_id);
    let assets = doc["assets"].as_array().unwrap();
    assert_eq!(assets.iter().filter(|a| a["inLibrary"] == true).count(), 1);
    assert!(
        assets
            .iter()
            .any(|a| a["id"] == old_id && a["inLibrary"] == false)
    );
    assert_eq!(rows(&store).len(), 1);
    let _ = std::fs::remove_dir_all(store.root);
}

#[test]
fn legacy_stacks_coalesce_to_latest_and_transfer_references_from_an_older_entry() {
    let store = setup();
    source(&store);
    let old = video(&store, "legacy-old", 0.5);
    store
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO previs_exports VALUES('p',?1,?2)",
            params![old.id, serde_json::to_string(&old).unwrap()],
        )
        .unwrap();
    let shared = sharing::set_shared(&store, "p", &old.id, true).unwrap();
    let old_id = shared["assetId"].as_str().unwrap().to_owned();
    edit(
        &store,
        |doc| {
            doc["nodes"][0]["references"] =
                json!([{"assetId":old_id,"purpose":"Motion reference","start":0,"end":0.5}])
        },
    );
    let latest = video(&store, "legacy-latest", 1.);
    store
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO previs_exports VALUES('p',?1,?2)",
            params![latest.id, serde_json::to_string(&latest).unwrap()],
        )
        .unwrap();
    assert!(export_store::normalize(&store, "p").unwrap());
    let saved = rows(&store);
    assert_eq!(saved.len(), 1);
    assert_eq!(saved[0].signature, "legacy-latest");
    let new_id = saved[0].shared_asset_id.as_ref().unwrap();
    let doc = document(&store);
    assert_eq!(doc["nodes"][0]["references"][0]["assetId"], *new_id);
    assert_eq!(doc["nodes"][0]["references"][0]["end"], 1.);
    assert_eq!(doc["assets"].as_array().unwrap().len(), 1);
    assert!(!std::path::Path::new(&old.asset.path).exists());
    assert!(!export_store::normalize(&store, "p").unwrap());
    assert_eq!(rows(&store)[0].shared_asset_id.as_ref(), Some(new_id));
    assert_eq!(document(&store)["assets"].as_array().unwrap().len(), 1);
    let _ = std::fs::remove_dir_all(store.root);
}

#[test]
fn a_stale_or_failed_replacement_leaves_the_valid_video_and_reference_intact() {
    let store = setup();
    source(&store);
    let first = export_store::replace(&store, "p", video(&store, "valid", 0.5)).unwrap();
    let shared = sharing::set_shared(&store, "p", &first.id, true).unwrap();
    edit(
        &store,
        |doc| {
            doc["nodes"][0]["references"] =
                json!([{"assetId":shared["assetId"],"purpose":"Motion","start":0,"end":0.5}])
        },
    );
    let before = document(&store);
    let mut stale = video(&store, "stale", 1.);
    stale.file_revision = 0;
    assert!(export_store::replace(&store, "p", stale).is_err());
    assert_eq!(rows(&store).len(), 1);
    assert_eq!(rows(&store)[0].signature, "valid");
    assert_eq!(document(&store), before);
    assert!(std::path::Path::new(&first.asset.path).exists());
    let mut invalid = video(&store, "missing", 1.);
    invalid.asset.path = "/does-not-exist.mp4".into();
    assert!(export_store::replace(&store, "p", invalid).is_err());
    assert_eq!(document(&store), before);
    assert_eq!(rows(&store)[0].signature, "valid");
    let _ = std::fs::remove_dir_all(store.root);
}
