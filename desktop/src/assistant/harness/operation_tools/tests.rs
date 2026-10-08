use super::*;
use crate::assistant::tool_schema;
pub(super) fn profile(id: &str) -> AgentProfile {
    crate::assistant::profiles::builtins()
        .into_iter()
        .find(|p| p.id == id)
        .unwrap()
}
// The shipped roster has no dedicated asset role; the asset-only boundary
// (project-assets without frames or production) is kept as an inline profile.
pub(super) fn asset_profile() -> AgentProfile {
    let mut p = profile("production");
    p.tool_ids = vec![
        "project-read".into(),
        "project-assets".into(),
        "media-generation".into(),
    ];
    p
}
// Exercise the real scheduler validation boundary before translating operations.
pub(super) fn decode(
    profile: &AgentProfile,
    name: &str,
    args: Value,
) -> Option<Result<Value, Value>> {
    let definitions: Vec<_> = tools(profile)
        .into_iter()
        .map(|tool| tool.definition)
        .collect();
    if !definitions.iter().any(|definition| definition.name == name) {
        return None;
    }
    let call = rig_core::message::ToolCall::from_wire(
        "test",
        rig_core::message::ToolFunction {
            name: name.into(),
            arguments: args.clone(),
        },
    );
    if let Some(error) = super::super::scheduler::validate(&definitions, &call) {
        return Some(Err(error));
    }
    super::decode(profile, name, args).map(Ok)
}
#[test]
fn production_has_direct_prompt_tools_without_storyboard_writes() {
    let p = profile("production");
    let available = tools(&p);
    // production has no project-shots/project-script/project-edit, so it still gets no
    // shot-structure or screenplay writer. Its generic node writer exists only for the
    // asset nodes project-assets covers, not for shot or screenplay design.
    for name in [
        "mstudio_edit",
        "mstudio_update_shot",
        "mstudio_update_shots",
        "mstudio_update_screenplay",
    ] {
        assert!(
            !available.iter().any(|t| t.definition.name == name),
            "{name}"
        );
    }
    let update_node = available
        .iter()
        .find(|t| t.definition.name == "mstudio_update_node")
        .expect("project-assets exposes the asset node writer");
    let properties = update_node.definition.parameters["properties"]
        .as_object()
        .unwrap();
    assert!(!properties.contains_key("shot"));
    assert!(!properties.contains_key("screenplay"));
    assert!(properties.contains_key("assetId"));
    let args = decode(
        &p,
        "mstudio_set_video_prompt",
        json!({"id":"s", "prompt":"English dialogue"}),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        args,
        json!({"action":"edit","operations":[{"op":"update_node","id":"s","shot":{"prompt":"English dialogue"}}]})
    );
    // The still prompt is the same direct edit; production owns both prompt kinds now.
    let args = decode(
        &p,
        "mstudio_set_image_prompt",
        json!({"id":"s", "framePrompt":"English still"}),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        args,
        json!({"action":"edit","operations":[{"op":"update_node","id":"s","shot":{"framePrompt":"English still"}}]})
    );
    for invalid in [
        json!({"id":"s","patch":{"prompt":"x"}}),
        json!({"id":"s","shot":{"prompt":"x"}}),
        json!({"id":"s","prompt":"x","dialogue":"x"}),
    ] {
        assert!(
            decode(&p, "mstudio_set_video_prompt", invalid)
                .unwrap()
                .is_err()
        );
    }
    assert!(decode(&p, "mstudio_update_shot", json!({"id":"s","dialogue":"x"})).is_none());
}
#[test]
fn generation_and_task_prompts_map_without_source_edits() {
    let p = profile("production");
    let args = decode(&p,"mstudio_generate_video",json!({"id":"s","prompt":"Final prompt","mode":"multi","parameters":{"duration":5,"resolution":"1080P"}})).unwrap().unwrap();
    assert_eq!(args["operations"][0]["op"], "request_generation");
    assert_eq!(args["operations"][0]["mediaKind"], "video");
    assert_eq!(args["operations"][0]["text"], "Final prompt");
    assert!(args["operations"][0].get("prompt").is_none());
    assert_eq!(
        decode(
            &p,
            "mstudio_update_generation",
            json!({"taskKey":"t","prompt":"Replacement"})
        )
        .unwrap()
        .unwrap(),
        json!({"action":"edit","operations":[{"op":"update_generation","taskKey":"t","text":"Replacement"}]})
    );
    assert!(
        decode(
            &p,
            "mstudio_generate_video",
            json!({"prompt":"x","mediaKind":"image"})
        )
        .unwrap()
        .is_err()
    );
    let asset = asset_profile();
    assert!(decode(&asset, "mstudio_generate_video", json!({"prompt":"x"})).is_none());
    let args = decode(
        &asset,
        "mstudio_generate_reference_image",
        json!({"prompt":"x","references":[]}),
    )
    .unwrap()
    .unwrap();
    assert_eq!(args["operations"][0]["generationPurpose"], "asset");
}
#[test]
fn every_role_has_unique_classified_tools_and_valid_internal_mapping() {
    for p in crate::assistant::profiles::builtins() {
        let mut names = std::collections::HashSet::new();
        for tool in tools(&p) {
            assert!(
                names.insert(tool.definition.name.clone()),
                "duplicate {}",
                tool.definition.name
            );
            assert!(is_edit(&tool.definition.name), "{}", tool.definition.name);
            assert!(tool.definition.parameters["properties"].get("op").is_none());
            assert!(
                tool.definition.parameters["properties"]
                    .get("operations")
                    .is_none()
            );
        }
    }
    assert!(is_edit("mstudio_edit")); // historical receipt recovery
    assert!(!is_edit("mstudio_inspect"));
    let mut p = profile("production");
    p.tool_ids = vec!["project-read".into(), "project-edit".into()];
    let args = decode(
        &p,
        "mstudio_update_shots",
        json!({"items":[{"id":"a","order":2},{"id":"b","order":1}]}),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        args,
        json!({"action":"edit","operations":[{"op":"update_node","id":"a","shot":{"order":2}},{"op":"update_node","id":"b","shot":{"order":1}}]})
    );
    assert!(super::super::schema::issues(&tool_schema::for_profile(&p), &args).is_empty());
    assert!(
        decode(&p, "mstudio_update_shots", json!({"items":[]}))
            .unwrap()
            .is_err()
    );
}
#[test]
fn creation_tools_fix_kind_and_keep_body_separate_from_shot_fields() {
    for (p, name, input, kind) in [
        (
            profile("concept"),
            "mstudio_add_shot",
            json!({"id":"new","title":"Shot","text":"Action","screenplayId":"script","scriptId":"paragraph","order":1,"duration":5}),
            "shot",
        ),
        (
            asset_profile(),
            "mstudio_add_asset",
            json!({"id":"new","title":"Asset","assetId":"media"}),
            "asset",
        ),
    ] {
        let args = decode(&p, name, input).unwrap().unwrap();
        assert_eq!(args["operations"][0]["kind"], kind);
        assert!(
            super::super::schema::issues(&tool_schema::for_profile(&p), &args).is_empty(),
            "{args}"
        );
        if kind == "shot" {
            assert_eq!(args["operations"][0]["text"], "Action");
            assert_eq!(args["operations"][0]["shot"]["duration"], 5);
            assert!(args["operations"][0]["shot"].get("text").is_none());
        }
    }
}
#[test]
fn direct_prompt_tool_commits_once_and_recovers_its_receipt() {
    use rig_core::message::{AssistantContent, Message, ToolCall, ToolFunction};
    let root = std::env::temp_dir().join(format!("mstudio-split-{}", mstudio::media::id()));
    let store = crate::database::Store::open(root.clone()).unwrap();
    let doc = json!({"id":"p","name":"Synthetic","revision":0,"brief":"","width":1280,"height":720,"fps":30,"assets":[],"nodes":[
        {"id":"script","kind":"screenplay","title":"Script","x":0,"y":0,"width":280,"height":218,"screenplay":{"script":[{"id":"paragraph","title":"Scene","action":"Action","duration":5}]}},
        {"id":"shot","kind":"shot","title":"Shot","text":"Preserved staging","x":320,"y":0,"width":280,"height":218,"shot":{"screenplayId":"script","scriptId":"paragraph","order":1,"duration":5,"dialogue":"Preserved dialogue","prompt":"Old"}}
    ],"clips":[],"tracks":[],"captions":[]});
    crate::projects::write_document(&store, doc, true).unwrap();
    let p = profile("production");
    crate::project_service::execute(
        &store,
        &p,
        "p",
        "turn",
        "read",
        json!({"action":"inspect","nodeIds":["shot","script"]}),
    )
    .unwrap();
    let input = json!({"id":"shot","prompt":"New video prompt"});
    let args = decode(&p, "mstudio_set_video_prompt", input.clone())
        .unwrap()
        .unwrap();
    let receipt =
        crate::project_service::execute(&store, &p, "p", "turn", "turn:call", args.clone())
            .unwrap();
    assert_eq!(receipt["outcome"], "committed", "{receipt}");
    assert_eq!(
        crate::project_service::execute(&store, &p, "p", "turn", "turn:call", args).unwrap(),
        receipt
    );
    let saved = crate::project_service::execute(
        &store,
        &p,
        "p",
        "turn",
        "verify",
        json!({"action":"inspect","nodeIds":["shot"],"fields":["prompt","dialogue","text"]}),
    )
    .unwrap()
    .to_string();
    for value in [
        "New video prompt",
        "Preserved dialogue",
        "Preserved staging",
    ] {
        assert!(saved.contains(value), "{saved}");
    }
    let call = ToolCall::from_wire(
        "call",
        ToolFunction {
            name: "mstudio_set_video_prompt".into(),
            arguments: input,
        },
    );
    let mut messages = vec![Message::Assistant {
        id: None,
        content: vec![AssistantContent::ToolCall(call)],
    }];
    super::super::session_recovery::repair(&store, "p", "turn", &mut messages);
    assert_eq!(messages.len(), 2);
    let restored = serde_json::to_string(&messages[1]).unwrap();
    assert!(restored.contains("committed"), "{restored}");
    assert!(!restored.contains("EFFECT_UNKNOWN"));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
