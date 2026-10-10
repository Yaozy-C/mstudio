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
                    min: Some(2.0),
                    max: Some(15.0),
                }),
                ..Default::default()
            }),
            ratio_from_reference: None,
            reference_limit: Some(4),
            reference_seconds: Some(10.),
        }),
    );
    validate(&accepted).unwrap();
    let limits = limits(&accepted);
    assert_eq!(limits.references, Some(4));
    assert_eq!(limits.seconds, Some(10.));

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
        // Configured limits must be positive.
        Capabilities {
            reference_limit: Some(0),
            ..Default::default()
        },
        Capabilities {
            reference_seconds: Some(0.),
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
                    min: Some(5.0),
                    max: Some(5.0),
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
        .is_err()
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
    assert_eq!(references[0].max, None);
    assert_eq!(limits(&gemini).references, None);
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
        ratio_from_reference: None,
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

#[path = "capabilities_tests_migration.rs"]
mod migration;

#[path = "capabilities_tests_regression.rs"]
mod regression;
