use super::*;
use serde_json::json;

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
