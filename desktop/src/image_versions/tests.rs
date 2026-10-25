use super::*;
use std::process::Command;

fn fixture() -> (Store, Value) {
    let (store, mut document, _) = crate::project_service::tests::fixture();
    let file = image(&store, "source", "red", 32);
    let asset = media::import(&file, &store.media_root()).unwrap();
    crate::project_storage::save_asset(&store, "p", &asset).unwrap();
    let mut value = json!(asset);
    value["inLibrary"] = json!(true);
    document["assets"] = json!([value]);
    document["nodes"][0]["assetId"] = json!(asset.id);
    document["nodes"][0]["kind"] = json!("asset");
    crate::projects::write_document(&store, document, false).unwrap();
    (store, value)
}
fn image(store: &Store, name: &str, color: &str, size: u32) -> PathBuf {
    let path = store.root.join(format!("{name}.png"));
    media::run(
        Command::new(media::binary("ffmpeg"))
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                &format!("color=c={color}:s={size}x{size}"),
                "-frames:v",
                "1",
                "-threads",
                "1",
            ])
            .arg(&path),
    )
    .unwrap();
    path
}
fn cleanup(store: Store) {
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn processing_and_selection_keep_one_stable_asset_and_independent_image_versions() {
    let (store, original) = fixture();
    let id = original["id"].as_str().unwrap();
    let before = store.project("p").unwrap();
    let source = json!({"inputAssetId":id,"operation":{"kind":"restore","denoise":0.2}});
    let output = image(&store, "blue", "blue", 64);
    let version = save(&store, "p", id, "Processed", &[output], &source)
        .unwrap()
        .remove(0);
    assert_ne!(version.id, id);
    assert_eq!(version.image.id, version.id);
    assert_eq!(store.assets().unwrap().len(), 1);
    let processed = store.project("p").unwrap();
    assert_eq!(processed["assets"].as_array().unwrap().len(), 1);
    assert_eq!(processed["assets"][0]["path"], original["path"]);
    assert_eq!(processed["assets"][0]["imageVersionCount"], 2);
    assert!(select(&store, "p", id, "missing").is_err());
    assert!(select(&store, "another-project", id, &version.id).is_err());
    assert!(input(&store, "p", &version.id, None).is_err());
    let selected = select(&store, "p", id, &version.id).unwrap();
    assert_eq!(selected["id"], id);
    assert_eq!(selected["name"], original["name"]);
    assert_eq!(selected["imageId"], id);
    assert_eq!(selected["imageVersionId"], version.id);
    assert_eq!(selected["path"], version.image.path);
    assert_eq!(selected["inLibrary"], true);
    assert_eq!(select(&store, "p", id, &version.id).unwrap(), selected);
    let after = store.project("p").unwrap();
    assert_eq!(after["assets"].as_array().unwrap().len(), 1);
    assert_eq!(after["nodes"], before["nodes"]);
    assert_eq!(after["clips"], before["clips"]);
    let resolved = crate::previs::media::resolved(&store, &after).unwrap();
    assert_eq!(resolved[0].id, id);
    assert_eq!(resolved[0].path, version.image.path);
    let pinned = crate::previs::media::resolved(&store, &before).unwrap();
    assert_eq!(pinned[0].path, original["path"].as_str().unwrap());
    assert_eq!(
        input(&store, "p", id, None).unwrap().0.path,
        version.image.path
    );
    assert_eq!(
        input(&store, "p", id, Some(&original_id(id)))
            .unwrap()
            .0
            .path,
        original["path"].as_str().unwrap()
    );
    // An unrelated save from an earlier snapshot cannot undo the selected version.
    crate::projects::save_merged(&store, before.clone(), before).unwrap();
    assert_eq!(
        store.project("p").unwrap()["assets"][0]["imageVersionId"],
        version.id
    );
    let green = image(&store, "green", "green", 64);
    save(&store, "p", id, "Second", &[green], &source).unwrap();
    assert_eq!(
        store.project("p").unwrap()["assets"][0]["imageVersionId"],
        version.id
    );
    assert_eq!(list(&store.db.lock().unwrap(), "p", id).unwrap().len(), 3);
    let root = store.root.clone();
    drop(store);
    let store = Store::open(root).unwrap();
    assert_eq!(input(&store, "p", id, None).unwrap().1, version.id);
    select(&store, "p", id, &original_id(id)).unwrap();
    assert_eq!(
        store.project("p").unwrap()["assets"][0]["path"],
        original["path"]
    );
    assert_eq!(store.assets().unwrap().len(), 1);
    crate::asset_library::promote(&store.db.lock().unwrap(), id).unwrap();
    crate::project_storage::remove(&store, "p").unwrap();
    assert!(!Path::new(&version.image.path).exists());
    assert!(Path::new(original["path"].as_str().unwrap()).exists());
    cleanup(store);
}

#[test]
fn original_draft_can_be_selected_without_creating_an_asset() {
    let (store, original) = fixture();
    let id = original["id"].as_str().unwrap();
    let mut doc = store.project("p").unwrap();
    doc["assets"][0]["inLibrary"] = json!(false);
    crate::projects::write_document(&store, doc, false).unwrap();
    let selected = select(&store, "p", id, &original_id(id)).unwrap();
    assert_eq!(selected["id"], id);
    assert_eq!(selected["path"], original["path"]);
    assert_eq!(selected["inLibrary"], true);
    assert_eq!(store.assets().unwrap().len(), 1);
    cleanup(store);
}

#[test]
fn old_derived_assets_and_nested_versions_migrate_back_to_the_original_asset() {
    let (store, original) = fixture();
    let id = original["id"].as_str().unwrap();
    let blue = media::import(&image(&store, "blue", "blue", 64), &store.media_root()).unwrap();
    let green = media::import(&image(&store, "green", "green", 64), &store.media_root()).unwrap();
    crate::project_storage::save_asset(&store, "p", &blue).unwrap();
    let mut doc = store.project("p").unwrap();
    let mut wrong = json!(blue);
    wrong["inLibrary"] = json!(true);
    doc["assets"].as_array_mut().unwrap().push(wrong);
    doc["nodes"][0]["assetId"] = json!(blue.id);
    doc["clips"] = json!([{"id":"clip","assetId":blue.id}]);
    crate::projects::write_document(&store, doc, false).unwrap();
    let db = store.db.lock().unwrap();
    db.execute_batch("DROP TABLE image_versions;
        CREATE TABLE image_versions(project_id TEXT,asset_id TEXT,id TEXT,data TEXT,provenance TEXT,created INTEGER DEFAULT 1,PRIMARY KEY(project_id,id));").unwrap();
    db.execute(
        "INSERT INTO image_versions(project_id,asset_id,id,data) VALUES('p',?1,?2,?3)",
        params![id, original_id(id), original.to_string()],
    )
    .unwrap();
    for (owner, file) in [(id, &blue), (blue.id.as_str(), &green)] {
        db.execute("INSERT INTO image_versions(project_id,asset_id,id,data,provenance) VALUES('p',?1,?2,?3,?4)",params![owner,file.id,serde_json::to_string(file).unwrap(),json!({"operation":{"kind":"restore"}}).to_string()]).unwrap();
    }
    init(&db).unwrap();
    init(&db).unwrap();
    let saved = project_service::load(&db, "p").unwrap();
    assert_eq!(saved["assets"].as_array().unwrap().len(), 1);
    assert_eq!(saved["assets"][0]["id"], id);
    assert_eq!(saved["assets"][0]["imageVersionId"], blue.id);
    assert_eq!(saved["assets"][0]["path"], blue.path);
    assert_eq!(saved["nodes"][0]["assetId"], id);
    assert_eq!(saved["clips"][0]["assetId"], id);
    let versions = list(&db, "p", id).unwrap();
    assert_eq!(versions.len(), 3);
    assert_eq!(versions[0].image.path, original["path"].as_str().unwrap());
    assert_eq!(versions[1].image.id, blue.id);
    assert_eq!(versions[2].image.id, green.id);
    assert!(Path::new(&blue.path).exists() && Path::new(&green.path).exists());
    drop(db);
    assert_eq!(store.assets().unwrap().len(), 1);
    cleanup(store);
}

#[test]
fn picture_ids_are_independent_of_asset_ids_and_cleanup_uses_the_picture_group() {
    let (store, original) = fixture();
    let id = original["id"].as_str().unwrap();
    let mut doc = store.project("p").unwrap();
    doc["assets"][0]["imageId"] = json!("picture");
    doc["assets"][0]["imageVersionId"] = json!(original_id("picture"));
    crate::projects::write_document(&store, doc, false).unwrap();
    let output = image(&store, "result", "blue", 64);
    let version = save(
        &store,
        "p",
        id,
        "Result",
        &[output],
        &json!({"operation":{"kind":"restore"}}),
    )
    .unwrap()
    .remove(0);
    let selected = select(&store, "p", id, &version.id).unwrap();
    assert_eq!(selected["id"], id);
    assert_eq!(selected["imageId"], "picture");
    let db = store.db.lock().unwrap();
    assert_eq!(list(&db, "p", "picture").unwrap()[0].image.id, "picture");
    assert!(list(&db, "p", id).is_err());
    drop(db);
    let before = store.project("p").unwrap();
    let mut after = before.clone();
    after["removedAssetIds"] = json!([id]);
    crate::projects::save_merged(&store, after, before).unwrap();
    assert!(!Path::new(&version.image.path).exists());
    assert_eq!(
        store
            .db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM image_versions", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    cleanup(store);
}
