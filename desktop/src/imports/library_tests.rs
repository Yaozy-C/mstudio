use super::*;

#[test]
fn global_upload_copies_source_and_persists_without_a_project() {
    let root = std::env::temp_dir().join(format!("mstudio-global-import-{}", media::id()));
    let store = Store::open(root.clone()).unwrap();
    let source = root.join("brief.txt");
    std::fs::write(&source, "global brand reference").unwrap();
    let asset = import_source_to(
        &store,
        None,
        &source,
        "brief.txt",
        &root.join("imports/stage"),
        false,
    )
    .unwrap();
    assert!(!asset.generated);
    assert!(Path::new(&asset.path).is_file());
    assert_ne!(asset.path, source.to_string_lossy());
    let db = store.db.lock().unwrap();
    assert_eq!(crate::asset_library::list(&db).unwrap()[0].id, asset.id);
    assert_eq!(
        db.query_row("SELECT count(*) FROM project_assets", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert!(
        import_source_to(
            &store,
            None,
            &source,
            "bad.exe",
            &root.join("imports/invalid"),
            false,
        )
        .is_err()
    );
    assert_eq!(crate::asset_library::list(&db).unwrap().len(), 1);
    drop(db);
    drop(store);
    let reopened = Store::open(root.clone()).unwrap();
    assert_eq!(
        crate::asset_library::list(&reopened.db.lock().unwrap()).unwrap()[0].id,
        asset.id
    );
    drop(reopened);
    std::fs::remove_dir_all(root).unwrap();
}
