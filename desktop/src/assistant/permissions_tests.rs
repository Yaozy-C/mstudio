use super::*;
use serde_json::json;

#[test]
fn resource_readers_follow_deliverable_capabilities() {
    for p in super::super::profiles::builtins() {
        assert_eq!(
            allows_resource_reader(&p, "mstudio_read_materials"),
            matches!(p.id.as_str(), "concept" | "writer" | "assets"),
            "{}",
            p.id
        );
        if matches!(p.id.as_str(), "concept" | "writer") {
            assert!(allows_resource_reader(&p, "mstudio_read_materials"));
            assert!(!allows_resource_reader(&p, "mstudio_read_assets"));
        }
        if matches!(p.id.as_str(), "director" | "editor") {
            assert!(allows_resource_reader(&p, "mstudio_read_assets"));
            assert!(!allows_resource_reader(&p, "mstudio_read_materials"));
        }
        if matches!(p.id.as_str(), "assets" | "image" | "production") {
            assert_eq!(
                allows_resource_reader(&p, "mstudio_read_materials"),
                p.id == "assets"
            );
            assert!(allows_resource_reader(&p, "mstudio_read_assets"));
        }
    }
    let custom = read_only_profile();
    assert!(!allows_resource_reader(&custom, "mstudio_read_materials"));
    assert!(allows_resource_reader(&custom, "mstudio_read_assets"));
    let mut custom = custom;
    custom.id = "custom-researcher".into();
    custom.tool_ids.push("material-read".into());
    assert!(allows_resource_reader(&custom, "mstudio_read_materials"));
    custom.tool_ids.retain(|id| id != "material-read");
    assert!(!allows_resource_reader(&custom, "mstudio_read_materials"));
}

// The optional read-only boundary remains supported for custom roles.
fn read_only_profile() -> AgentProfile {
    let mut p = super::super::profiles::builtins()
        .into_iter()
        .find(|a| a.id == "editor")
        .unwrap();
    p.tool_ids = vec!["project-read".into()];
    p
}
fn asset_only_profile() -> AgentProfile {
    super::super::profiles::builtins()
        .into_iter()
        .find(|a| a.id == "assets")
        .unwrap()
}
#[test]
fn asset_specialist_can_set_direct_shot_references_but_not_shot_design() {
    let asset = asset_only_profile();
    assert!(asset_only(&asset));
    assert!(
        super::super::profiles::builtins()
            .iter()
            .all(|p| asset_only(p) == (p.id == "assets")),
        "only the asset specialist has the asset-only restriction"
    );
    let doc = json!({"nodes":[{"id":"shot","kind":"shot"}]});
    assert!(
        validate(
            &asset,
            &json!({"operations":[{"op":"set_references","id":"shot","references":[]}]}),
            &doc
        )
        .is_ok()
    );
    assert!(
        validate(
            &asset,
            &json!({"operations":[{"op":"update_node","id":"shot","text":"changed"}]}),
            &doc
        )
        .is_err()
    );
}
#[test]
fn task_edits_use_existing_media_roles_and_actual_task_kind() {
    let builtin = |id: &str| {
        super::super::profiles::builtins()
            .into_iter()
            .find(|a| a.id == id)
            .unwrap()
    };
    let doc = json!({"nodes":[],"production":{"drafts":{"image":{"kind":"image"},"video":{"kind":"video"}}}});
    // production now holds project-production, so it may revise both task kinds. A profile
    // limited to project-frames may only revise image tasks, and a read-only profile none.
    let mut frame_only = builtin("production");
    frame_only.tool_ids = vec![
        "project-read".into(),
        "project-frames".into(),
        "media-generation".into(),
    ];
    for operation in ["update_generation", "regenerate_generation"] {
        for (agent, key, allowed) in [
            (builtin("production"), "video", true),
            (builtin("production"), "image", false),
            (frame_only.clone(), "image", true),
            (frame_only.clone(), "video", false),
            (read_only_profile(), "image", false),
            (builtin("production"), "missing", false),
        ] {
            let result = validate(
                &agent,
                &json!({"operations":[{"op":operation,"taskKey":key,"text":"new"}]}),
                &doc,
            );
            assert_eq!(result.is_ok(), allowed, "{}: {operation} {key}", agent.id);
        }
    }
}
#[test]
fn specialists_cannot_cross_edit_boundaries() {
    let agents = super::super::profiles::builtins();
    let agent = |id: &str| agents.iter().find(|a| a.id == id).unwrap().clone();
    let doc = json!({"nodes":[{"id":"p","kind":"screenplay"},{"id":"s","kind":"shot"}]});
    let check =
        |p: &AgentProfile, op: Value| validate(p, &json!({"operations":[op]}), &doc).is_ok();
    let read_only = read_only_profile();
    assert!(!check(
        &agent("concept"),
        json!({"op":"update_node","id":"p","text":"concept"})
    ));
    assert!(check(
        &agent("writer"),
        json!({"op":"update_node","id":"p","screenplay":{"script":[{"id":"para","action":"Story"}]}})
    ));
    // concept owns shot design through project-shots: it edits shot links, dialogue and
    // staging text, but it has no media-generation tool.
    assert!(check(
        &agent("director"),
        json!({"op":"update_node","id":"s","shot":{"scriptId":"para"}})
    ));
    assert!(check(
        &agent("director"),
        json!({"op":"update_node","id":"s","text":"rewrite"})
    ));
    assert!(check(
        &agent("director"),
        json!({"op":"update_node","id":"s","shot":{"dialogue":"hello"}})
    ));
    assert!(!check(
        &agent("director"),
        json!({"op":"request_generation","id":"s"})
    ));
    // production now owns both prompt kinds through project-frames and project-production,
    // so writing frame prompts and shot frames is no longer another role's boundary.
    assert!(check(
        &agent("image"),
        json!({"op":"update_node","id":"s","shot":{"framePrompt":"Image description"}})
    ));
    assert!(check(
        &agent("image"),
        json!({"op":"update_node","id":"s","shot":{"frames":[]}})
    ));
    assert!(check(
        &agent("image"),
        json!({"op":"update_node","id":"s","shot":{"framePrompt":"Image description","frames":[]}})
    ));
    assert!(check(
        &agent("production"),
        json!({"op":"update_node","id":"s","shot":{"prompt":"generate"}})
    ));
    assert!(check(
        &agent("image"),
        json!({"op":"request_generation","id":"s","text":"Close-up","mediaKind":"image"})
    ));
    // It has no project-shots/project-edit, so shot structure, timing and staging text stay
    // outside its scope, and it has no timeline tool.
    assert!(!check(
        &agent("production"),
        json!({"op":"update_node","id":"s","shot":{"scriptId":"other"}})
    ));
    assert!(!check(
        &agent("production"),
        json!({"op":"update_node","id":"s","shot":{"duration":1}})
    ));
    assert!(!check(
        &agent("production"),
        json!({"op":"update_node","id":"s","shot":{"dialogue":"hello"}})
    ));
    assert!(!check(
        &agent("production"),
        json!({"op":"update_node","id":"s","text":"remove main action"})
    ));
    assert!(!check(
        &agent("production"),
        json!({"op":"append_clip","assetId":"a"})
    ));
    // editor now owns cutting, colour and transitions together.
    assert!(check(
        &agent("editor"),
        json!({"op":"set_transition","fromClipId":"a","id":"b","kind":"fade","duration":0.5})
    ));
    assert!(check(
        &agent("editor"),
        json!({"op":"update_clip","id":"b","visual":{"temperature":-0.2}})
    ));
    assert!(check(
        &agent("editor"),
        json!({"op":"update_clip","id":"c","speed":1.5})
    ));
    // A read-only profile cannot join or retime clips.
    assert!(!check(
        &read_only,
        json!({"op":"set_transition","fromClipId":"a","id":"b","kind":"fade","duration":0.5})
    ));
    assert!(!check(
        &read_only,
        json!({"op":"update_clip","id":"c","speed":1.5})
    ));
    assert!(check(
        &agent("coordinator"),
        json!({"op":"set_creation","essential":"must see entry"})
    ));
    assert!(!check(
        &agent("coordinator"),
        json!({"op":"update_node","id":"p","text":"rewrite"})
    ));
}

#[test]
fn task_archival_allows_coordination_without_media_authoring() {
    let doc = json!({"production":{"drafts":{"image":{"kind":"image"},"video":{"kind":"video"}}}});
    for (role, key, allowed) in [
        ("coordinator", "image", true),
        ("coordinator", "video", true),
        ("image", "image", true),
        ("image", "video", false),
        ("production", "video", true),
        ("production", "image", false),
        ("writer", "image", false),
        ("director", "video", false),
        ("coordinator", "missing", false),
    ] {
        let p = super::super::profiles::builtins()
            .into_iter()
            .find(|p| p.id == role)
            .unwrap();
        let args =
            json!({"operations":[{"op":"archive_generation","taskKey":key,"archived":true}]});
        assert_eq!(validate(&p, &args, &doc).is_ok(), allowed, "{role}: {key}");
    }
    assert!(!allows_operation(
        &read_only_profile(),
        "archive_generation"
    ));
}
