use super::*;
use serde_json::json;
#[test]
fn asset_specialist_can_set_direct_shot_references_but_not_shot_design() {
    let agents = super::super::profiles::builtins();
    let asset = agents.iter().find(|a| a.id == "asset-designer").unwrap();
    let doc = json!({"nodes":[{"id":"shot","kind":"shot"}]});
    assert!(
        validate(
            asset,
            &json!({"operations":[{"op":"set_references","id":"shot","references":[]}]}),
            &doc
        )
        .is_ok()
    );
    assert!(
        validate(
            asset,
            &json!({"operations":[{"op":"update_node","id":"shot","text":"changed"}]}),
            &doc
        )
        .is_err()
    );
}
#[test]
fn task_edits_use_existing_media_roles_and_actual_task_kind() {
    let agents = super::super::profiles::builtins();
    let doc = json!({"nodes":[],"production":{"drafts":{"image":{"kind":"image"},"video":{"kind":"video"}}}});
    for operation in ["update_generation", "regenerate_generation"] {
        for (agent, key, allowed) in [
            ("production", "video", true),
            ("storyboard-artist", "image", true),
            ("storyboard-artist", "video", false),
            ("reviewer", "image", false),
            ("production", "missing", false),
        ] {
            let result = validate(
                agents.iter().find(|a| a.id == agent).unwrap(),
                &json!({"operations":[{"op":operation,"taskKey":key,"text":"new"}]}),
                &doc,
            );
            assert_eq!(result.is_ok(), allowed, "{agent}: {operation} {key}");
        }
    }
}
#[test]
fn specialists_cannot_cross_edit_boundaries() {
    let agents = super::super::profiles::builtins();
    let doc = json!({"nodes":[{"id":"p","kind":"screenplay"},{"id":"s","kind":"shot"}]});
    let check = |id: &str, op: Value| {
        validate(
            agents.iter().find(|a| a.id == id).unwrap(),
            &json!({"operations":[op]}),
            &doc,
        )
        .is_ok()
    };
    assert!(!check(
        "concept",
        json!({"op":"update_node","id":"p","text":"concept"})
    ));
    assert!(check(
        "concept",
        json!({"op":"update_node","id":"p","screenplay":{"script":[{"id":"para","action":"Story"}]}})
    ));
    assert!(check(
        "storyboard",
        json!({"op":"update_node","id":"s","shot":{"scriptId":"para"}})
    ));
    assert!(!check(
        "production",
        json!({"op":"update_node","id":"s","shot":{"framePrompt":"Image description"}})
    ));
    assert!(!check(
        "production",
        json!({"op":"update_node","id":"s","shot":{"frames":[]}})
    ));
    assert!(check(
        "storyboard-artist",
        json!({"op":"update_node","id":"s","shot":{"framePrompt":"Image description","frames":[]}})
    ));
    assert!(!check(
        "production",
        json!({"op":"update_node","id":"s","shot":{"scriptId":"other"}})
    ));
    assert!(!check(
        "concept",
        json!({"op":"update_node","id":"s","text":"rewrite"})
    ));
    assert!(check(
        "storyboard",
        json!({"op":"update_node","id":"s","shot":{"dialogue":"hello"}})
    ));
    assert!(!check(
        "storyboard",
        json!({"op":"request_generation","id":"s"})
    ));
    assert!(!check(
        "storyboard",
        json!({"op":"request_generation","id":"s"})
    ));
    assert!(check(
        "production",
        json!({"op":"request_generation","id":"s","text":"Close-up","mediaKind":"image"})
    ));
    assert!(check(
        "production",
        json!({"op":"update_node","id":"s","shot":{"prompt":"generate"}})
    ));
    assert!(!check(
        "production",
        json!({"op":"update_node","id":"s","text":"remove main action"})
    ));
    assert!(!check(
        "production",
        json!({"op":"update_node","id":"s","shot":{"duration":1}})
    ));
    assert!(!check(
        "production",
        json!({"op":"append_clip","assetId":"a"})
    ));
    assert!(check(
        "transition-designer",
        json!({"op":"set_transition","fromClipId":"a","id":"b","kind":"fade","duration":0.5})
    ));
    assert!(check(
        "colorist",
        json!({"op":"update_clip","id":"b","visual":{"temperature":-0.2}})
    ));
    assert!(!check(
        "reviewer",
        json!({"op":"set_transition","fromClipId":"a","id":"b","kind":"fade","duration":0.5})
    ));
    assert!(check(
        "editor",
        json!({"op":"update_clip","id":"c","speed":1.5})
    ));
    assert!(!check(
        "reviewer",
        json!({"op":"update_clip","id":"c","speed":1.5})
    ));
    assert!(check(
        "coordinator",
        json!({"op":"set_creation","essential":"must see entry"})
    ));
    assert!(!check(
        "coordinator",
        json!({"op":"update_node","id":"p","text":"rewrite"})
    ));
}
