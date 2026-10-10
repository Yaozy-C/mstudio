use super::*;

#[test]
fn migration_stamps_legacy_models_once_and_keeps_explicit_declarations() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    let h3 = model("fal", "minimax/h3/reference-to-video", None);
    let untouched = model(
        "fal",
        "fal-ai/flux/schnell",
        Some(Capabilities {
            reference_limit: Some(2),
            ..Default::default()
        }),
    );
    crate::models::media::write(&db, &[h3.clone(), untouched.clone()]).unwrap();

    migrate(&db).unwrap();
    let stored = crate::models::media::read(&db).unwrap();
    let migrated = stored.iter().find(|m| m.id == "model").unwrap();
    assert!(migrated.capabilities.is_some());
    assert_eq!(
        migrated.capabilities.as_ref().unwrap().reference_limit,
        Some(12)
    );
    assert_eq!(
        migrated
            .capabilities
            .as_ref()
            .unwrap()
            .references
            .as_ref()
            .unwrap()
            .len(),
        3
    );
    validate(migrated).unwrap();

    // An endpoint with no legacy knowledge keeps protocol defaults only.
    let plain = model("fal", "fal-ai/wan/v2.7/image-to-video", None);
    crate::models::media::write(&db, std::slice::from_ref(&plain)).unwrap();
    migrate(&db).unwrap();
    let stored = crate::models::media::read(&db).unwrap();
    assert!(stored[0].capabilities.is_none());
    assert!(legacy_declaration("fal", "fal-ai/wan/v2.7/image-to-video").is_none());
}

#[test]
fn legacy_table_matches_the_capabilities_the_old_code_hardcoded() {
    let h3_frames = legacy_declaration("fal", "minimax/h3/image-to-video").unwrap();
    let references = h3_frames.references.unwrap();
    assert_eq!(
        references
            .iter()
            .map(|reference| (reference.key.as_str(), reference.role.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("/image_url", "first-frame"),
            ("/end_image_url", "last-frame"),
            ("/target_audio_url", "reference"),
        ]
    );
    let controls = h3_frames.controls.unwrap();
    assert!(controls.aspect_ratio.is_none());
    let duration = controls.duration.unwrap();
    assert_eq!(duration.min, Some(5.0));
    assert_eq!(duration.max, Some(15.0));

    let edit = legacy_declaration("fal", "openai/gpt-image-2.5/flare/edit").unwrap();
    assert!(edit.references.unwrap()[0].required);
    assert_eq!(
        edit.controls.unwrap().image_size.unwrap().path,
        "/image_size"
    );

    assert!(legacy_declaration("gemini-native", "gemini-3.1-flash-image").is_none());
    assert!(legacy_declaration("fal", "fal-ai/flux/schnell").is_none());
}
