use super::tests::{decode, profile};
use super::*;
#[test]
fn edits_and_removals_are_object_specific_and_reject_wrappers() {
    let mut p = profile("director");
    p.tool_ids = vec!["project-read".into(), "project-edit".into()];
    for old in [
        "mstudio_update_node",
        "mstudio_remove_node",
        "mstudio_update_clip",
        "mstudio_edit",
    ] {
        assert!(decode(&p, old, json!({"id":"s"})).is_none(), "{old}");
    }
    for (name, kind, input) in [
        (
            "mstudio_update_shot_design",
            "shot",
            json!({"id":"s","text":"New staging"}),
        ),
        (
            "mstudio_update_screenplay_info",
            "screenplay",
            json!({"id":"script","title":"New title"}),
        ),
        (
            "mstudio_update_asset_node",
            "asset",
            json!({"id":"node","assetId":"media"}),
        ),
        ("mstudio_remove_shot", "shot", json!({"id":"s"})),
        (
            "mstudio_remove_screenplay",
            "screenplay",
            json!({"id":"script"}),
        ),
        ("mstudio_remove_asset_node", "asset", json!({"id":"node"})),
    ] {
        let internal = decode(&p, name, input).unwrap().unwrap();
        assert_eq!(internal["operations"][0]["expectedKind"], kind);
        assert!(
            super::super::schema::issues(
                &crate::assistant::tool_schema::for_profile(&p),
                &internal
            )
            .is_empty(),
            "{internal}"
        );
    }
    assert!(
        decode(
            &p,
            "mstudio_update_shot_design",
            json!({"id":"s","shot":{"text":"x"}})
        )
        .unwrap()
        .is_err()
    );
    assert!(
        decode(&p, "mstudio_remove_shot", json!({"id":"s","kind":"asset"}))
            .unwrap()
            .is_err()
    );
    assert!(
        decode(
            &profile("production"),
            "mstudio_remove_shot",
            json!({"id":"s"})
        )
        .is_none()
    );
    let doc = json!({"nodes":[{"id":"wrong","kind":"asset"}]});
    for name in ["mstudio_update_shot_design", "mstudio_remove_shot"] {
        let input = if name.contains("update") {
            json!({"id":"wrong","text":"x"})
        } else {
            json!({"id":"wrong"})
        };
        let internal = decode(&p, name, input).unwrap().unwrap();
        assert!(crate::assistant::permissions::validate(&p, &internal, &doc).is_err());
    }
}
#[test]
fn clip_tools_accept_only_their_direct_fields_and_preserve_internal_transaction_shape() {
    let p = profile("editor");
    for (name, input, expected) in [
        (
            "mstudio_trim_clip",
            json!({"id":"c","trimIn":1,"trimOut":3}),
            json!({"id":"c","op":"update_clip","trimIn":1,"trimOut":3}),
        ),
        (
            "mstudio_set_clip_audio",
            json!({"id":"c","volume":0.7}),
            json!({"id":"c","op":"update_clip","volume":0.7}),
        ),
        (
            "mstudio_set_clip_visual",
            json!({"id":"c","brightness":0.1}),
            json!({"id":"c","op":"update_clip","visual":{"brightness":0.1}}),
        ),
        (
            "mstudio_set_clip_grade",
            json!({"id":"c","exposure":0.4,"contrast":10}),
            json!({"id":"c","op":"update_clip","visual":{"grade":{"exposure":0.4,"contrast":10}}}),
        ),
        (
            "mstudio_clear_clip_grade",
            json!({"id":"c"}),
            json!({"id":"c","op":"update_clip","visual":{"grade":null}}),
        ),
        (
            "mstudio_clear_clip_visual",
            json!({"id":"c"}),
            json!({"id":"c","op":"update_clip","visual":null}),
        ),
    ] {
        let internal = decode(&p, name, input).unwrap().unwrap();
        assert_eq!(internal["operations"][0], expected);
        assert!(
            super::super::schema::issues(
                &crate::assistant::tool_schema::for_profile(&p),
                &internal
            )
            .is_empty(),
            "{name}: {internal}"
        );
    }
    for (name, args) in [
        (
            "mstudio_trim_clip",
            json!({"id":"c","visual":{"grade":{"exposure":1}}}),
        ),
        (
            "mstudio_set_clip_grade",
            json!({"id":"c","visual":{"grade":{"exposure":1}}}),
        ),
        ("mstudio_set_clip_audio", json!({"id":"c","trimIn":2})),
    ] {
        assert!(decode(&p, name, args).unwrap().is_err());
    }
}
