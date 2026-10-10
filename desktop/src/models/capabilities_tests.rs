use super::*;
use crate::models::media::MediaModel;
use serde_json::json;

fn model(plugin: &str, endpoint: &str, capabilities: Option<Capabilities>) -> MediaModel {
    MediaModel {
        id: "model".into(),
        name: "Model".into(),
        kind: "video".into(),
        plugin: plugin.into(),
        endpoint: endpoint.into(),
        params: json!({}),
        enabled: true,
        http: None,
        capabilities,
        connection_id: None,
        has_key: false,
    }
}

#[test]
fn declared_pointers_roles_and_limits_are_validated() {
    let accepted = model(
        "fal",
        "vendor/model",
        Some(Capabilities {
            references: Some(vec![ReferenceDeclaration {
                key: "/input/img_url".into(),
                kind: "image".into(),
                role: "first-frame".into(),
                multiple: false,
                required: false,
                max: Some(2),
            }]),
            controls: Some(Controls {
                duration: Some(ControlDeclaration {
                    path: "/parameters/duration".into(),
                    values: None,
                    min: Some(2),
                    max: Some(15),
                }),
                ..Default::default()
            }),
            reference_limit: Some(4),
            reference_seconds: Some(10.),
        }),
    );
    validate(&accepted).unwrap();
    let limits = limits(&accepted);
    assert_eq!(limits.references, 4);
    assert_eq!(limits.seconds, 10.);

    for broken in [
        // A pointer must be a JSON Pointer, and a field may not repeat.
        Capabilities {
            references: Some(vec![ReferenceDeclaration {
                key: "image_url".into(),
                kind: "image".into(),
                role: "reference".into(),
                multiple: false,
                required: false,
                max: None,
            }]),
            ..Default::default()
        },
        Capabilities {
            references: Some(vec![
                ReferenceDeclaration {
                    key: "/image_url".into(),
                    kind: "image".into(),
                    role: "reference".into(),
                    multiple: false,
                    required: false,
                    max: None,
                },
                ReferenceDeclaration {
                    key: "/image_url".into(),
                    kind: "image".into(),
                    role: "reference".into(),
                    multiple: false,
                    required: false,
                    max: None,
                },
            ]),
            ..Default::default()
        },
        // Unknown roles and kinds cannot reach a request encoder.
        Capabilities {
            references: Some(vec![ReferenceDeclaration {
                key: "/image_url".into(),
                kind: "image".into(),
                role: "first_frame".into(),
                multiple: false,
                required: false,
                max: None,
            }]),
            ..Default::default()
        },
        Capabilities {
            references: Some(vec![ReferenceDeclaration {
                key: "/clip".into(),
                kind: "clip".into(),
                role: "reference".into(),
                multiple: false,
                required: false,
                max: None,
            }]),
            ..Default::default()
        },
        // Limits can only lower the system ceiling.
        Capabilities {
            reference_limit: Some(20),
            ..Default::default()
        },
        Capabilities {
            reference_seconds: Some(60.),
            ..Default::default()
        },
        // Enum controls need values; duration and custom size must not carry any.
        Capabilities {
            controls: Some(Controls {
                aspect_ratio: Some(ControlDeclaration {
                    path: "/aspect_ratio".into(),
                    values: None,
                    min: None,
                    max: None,
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
        Capabilities {
            controls: Some(Controls {
                duration: Some(ControlDeclaration {
                    path: "/duration".into(),
                    values: Some(vec!["5".into()]),
                    min: Some(5),
                    max: Some(5),
                }),
                ..Default::default()
            }),
            ..Default::default()
        },
    ] {
        assert!(
            validate(&model("fal", "vendor/model", Some(broken.clone()))).is_err(),
            "{broken:?} 应被拒绝"
        );
    }
}

#[test]
fn protocols_that_build_their_own_body_reject_foreign_references() {
    let template = Capabilities {
        references: Some(vec![ReferenceDeclaration {
            key: "/image_url".into(),
            kind: "image".into(),
            role: "reference".into(),
            multiple: true,
            required: false,
            max: None,
        }]),
        ..Default::default()
    };
    assert!(
        validate(&model(
            "http-json",
            "https://example.com/generate",
            Some(template.clone())
        ))
        .is_err()
    );
    assert!(
        validate(&model(
            "gemini-native",
            "https://generativelanguage.googleapis.com",
            Some(template.clone())
        ))
        .is_ok()
    );
    let video = Capabilities {
        references: Some(vec![ReferenceDeclaration {
            key: "/clip".into(),
            kind: "video".into(),
            role: "reference".into(),
            multiple: false,
            required: false,
            max: None,
        }]),
        ..Default::default()
    };
    assert!(
        validate(&model(
            "gemini-native",
            "https://generativelanguage.googleapis.com",
            Some(video.clone())
        ))
        .is_err()
    );
    assert!(validate(&model("codex-image", "codex://local/images", Some(video))).is_err());
}

#[test]
fn protocol_defaults_still_answer_for_builders_that_own_the_body() {
    let gemini = model(
        "gemini-native",
        "https://generativelanguage.googleapis.com",
        None,
    );
    let references = effective_references(&gemini);
    assert_eq!(references.len(), 1);
    assert_eq!(references[0].key, "/image_urls");
    assert_eq!(references[0].max, Some(9));
    assert_eq!(limits(&gemini).references, MAX_REFERENCES);
    // fal and custom HTTP imply nothing: the declaration is the only source.
    assert!(effective_references(&model("fal", "fal-ai/flux/schnell", None)).is_empty());
    assert!(
        effective_references(&model("http-json", "https://example.com/generate", None)).is_empty()
    );
}

#[test]
fn the_request_body_is_checked_against_the_declaration() {
    let declared = Capabilities {
        references: Some(vec![
            ReferenceDeclaration {
                key: "/input/img_url".into(),
                kind: "image".into(),
                role: "first-frame".into(),
                multiple: false,
                required: true,
                max: None,
            },
            ReferenceDeclaration {
                key: "/image_urls".into(),
                kind: "image".into(),
                role: "reference".into(),
                multiple: true,
                required: false,
                max: Some(2),
            },
        ]),
        reference_limit: Some(3),
        reference_seconds: None,
        controls: None,
    };
    let declared_model = model("fal", "vendor/model", Some(declared));
    validate_input(
        &declared_model,
        &json!({"input": {"img_url": "https://example.com/f.png"}}),
    )
    .unwrap();
    validate_input(
        &declared_model,
        &json!({
            "input": {"img_url": "https://example.com/f.png"},
            "image_urls": ["https://example.com/a.png", "https://example.com/b.png"]
        }),
    )
    .unwrap();
    for rejected in [
        // Missing required reference.
        json!({"image_urls": ["https://example.com/a.png"]}),
        // More than the declared per-field maximum.
        json!({
            "input": {"img_url": "https://example.com/f.png"},
            "image_urls": ["a", "b", "c"]
        }),
        // A single-valued field must not receive an array.
        json!({"input": {"img_url": ["a", "b"]}}),
    ] {
        assert!(
            validate_input(&declared_model, &rejected).is_err(),
            "{rejected}"
        );
    }
    // An undeclared protocol keeps the system total cap and no field vocabulary.
    let plain = model("fal", "fal-ai/flux/schnell", None);
    validate_input(
        &plain,
        &json!({"prompt": "p", "image_urls": ["a", "b", "c"]}),
    )
    .unwrap();
}

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
    assert_eq!(duration.min, Some(5));
    assert_eq!(duration.max, Some(15));

    let edit = legacy_declaration("fal", "openai/gpt-image-2.5/flare/edit").unwrap();
    assert!(edit.references.unwrap()[0].required);
    assert_eq!(
        edit.controls.unwrap().image_size.unwrap().path,
        "/image_size"
    );

    assert!(legacy_declaration("gemini-native", "gemini-3.1-flash-image").is_none());
    assert!(legacy_declaration("fal", "fal-ai/flux/schnell").is_none());
}
