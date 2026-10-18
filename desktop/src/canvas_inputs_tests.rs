use super::validate_project;
use mstudio::model::Asset;
use serde_json::json;

#[test]
fn video_submission_resolves_private_previs_without_collecting_it() {
    let (store, mut doc, _) = crate::project_service::tests::fixture();
    doc["nodes"][0]["kind"] = json!("shot");
    doc["nodes"][0]["shot"] = json!({"frames":[]});
    crate::projects::write_document(&store, doc.clone(), false).unwrap();
    let path = store.media_root().join("previs.mp4");
    std::fs::write(&path, b"registered media").unwrap();
    let asset: Asset = serde_json::from_value(json!({
        "id":"previs","name":"Previs","kind":"video","path":path,
        "preview":"","duration":5.,"width":720,"height":1280,"hasAudio":false
    }))
    .unwrap();
    let submitted = json!({
        "id":"a","kind":"shot","references":[{"assetId":"previs"}],
        "canvasGeneration":{"task":{"kind":"video","ownerId":"a",
            "inputs":[{"assetId":"previs","role":"video-reference"}]}}
    });
    {
        let db = store.db.lock().unwrap();
        crate::previs::media::register(&db, "p", &asset).unwrap();
        let before = crate::project_service::load(&db, "p").unwrap();
        let mut after = before.clone();
        after["nodes"][0]["references"] = submitted["references"].clone();
        crate::project_service::persist(&db, &before, &after).unwrap();
        // The submission gate must hydrate metadata that is absent from raw JSON.
        validate_project(&db, "p", &submitted).unwrap();
        let raw: String = db
            .query_row("SELECT document FROM projects WHERE id='p'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let saved: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert!(saved.get("previsAssets").is_none());
        assert!(saved["assets"].as_array().unwrap().is_empty());
        for table in ["assets", "project_assets", "jobs"] {
            let count: i64 = db
                .query_row(&format!("SELECT count(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "validation must not create {table} records");
        }

        // A different project's private media is not a valid input.
        doc["id"] = json!("other");
        db.execute(
            "INSERT INTO projects VALUES('other','Other',?1,0)",
            [doc.to_string()],
        )
        .unwrap();
        assert!(validate_project(&db, "other", &submitted).is_err());

        // Missing files fail clearly, then recover when restored at the same path.
        std::fs::remove_file(&path).unwrap();
        assert!(
            validate_project(&db, "p", &submitted)
                .unwrap_err()
                .to_string()
                .contains("文件缺失")
        );
        std::fs::write(&path, b"restored media").unwrap();
        validate_project(&db, "p", &submitted).unwrap();

        // Video reference support must not bypass storyboard frame validation.
        let mut frame = submitted.clone();
        frame["canvasGeneration"]["task"]["inputs"][0]["frame"] = json!(true);
        assert!(validate_project(&db, "p", &frame).is_err());
        let mut inconsistent = submitted.clone();
        inconsistent["references"] = json!([]);
        assert!(validate_project(&db, "p", &inconsistent).is_err());
        db.execute("DELETE FROM previs_media WHERE project_id='p'", [])
            .unwrap();
        assert!(validate_project(&db, "p", &submitted).is_err());
    }
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
