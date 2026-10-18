use super::*;
use serde_json::json;
use std::fs;

#[test]
fn referenced_binary_files_follow_storage_relocation() {
    let root = std::env::temp_dir().join(format!("mstudio-media-move-{}", mstudio::media::id()));
    let store = Store::open(root.join("app")).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','p','{}',0)", [])
        .unwrap();
    let job = json!({"id":"j","projectId":"p","input":{"image":format!("data:image/png;base64,{}",crate::database::media_tests::PNG)}});
    crate::jobs::save(&store, &job).unwrap();
    let target = root.join("library");
    let result = migration::migrate(&store, &target, |_| Ok(())).unwrap();
    assert_eq!(result["files"], 1);
    assert_eq!(crate::jobs::get(&store, "j").unwrap(), job);
    assert_eq!(
        std::fs::read_dir(store.root.join("reference-media"))
            .unwrap()
            .count(),
        0
    );
    drop(store);
    let store = Store::open(root.join("app")).unwrap();
    assert_eq!(crate::jobs::get(&store, "j").unwrap(), job);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn private_previs_snapshots_move_and_restore_without_becoming_library_assets() {
    let root = std::env::temp_dir().join(format!("mstudio-previs-move-{}", mstudio::media::id()));
    let store = Store::open(root.join("app")).unwrap();
    crate::projects::write_document(
        &store,
        json!({"id":"p","name":"P","assets":[],"nodes":[],"clips":[]}),
        true,
    )
    .unwrap();
    let directory = store.media_root().join("exports/previs-move");
    fs::create_dir_all(&directory).unwrap();
    let path = directory.join("video.mp4");
    fs::write(&path, "immutable video").unwrap();
    let asset: mstudio::model::Asset = serde_json::from_value(json!({"id":"private","name":"Previs","kind":"video","path":path,"preview":"","duration":1,"width":64,"height":64,"hasAudio":false,"generated":true})).unwrap();
    crate::previs::media::register(&store.db.lock().unwrap(), "p", &asset).unwrap();
    crate::project_storage::track_file(&store, "p", &directory).unwrap();
    assert!(!statuses(&store, std::slice::from_ref(&asset.id)).unwrap()[&asset.id]);
    migration::migrate(&store, &root.join("library"), |_| Ok(())).unwrap();
    let doc = crate::project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    let moved = Path::new(doc["previsAssets"][0]["path"].as_str().unwrap());
    assert_eq!(fs::read_to_string(moved).unwrap(), "immutable video");
    assert!(!path.exists());
    assert!(doc["assets"].as_array().unwrap().is_empty());
    assert!(store.assets().unwrap().is_empty());
    fs::remove_file(moved).unwrap();
    assert!(statuses(&store, std::slice::from_ref(&asset.id)).unwrap()[&asset.id]);
    fs::write(moved, "restored").unwrap();
    assert!(!statuses(&store, std::slice::from_ref(&asset.id)).unwrap()[&asset.id]);
    crate::project_storage::remove(&store, "p").unwrap();
    assert!(!moved.exists());
    let _ = fs::remove_dir_all(root);
}
