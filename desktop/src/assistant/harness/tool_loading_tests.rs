use super::*;
fn profile(id: &str) -> crate::assistant::profiles::AgentProfile {
    crate::assistant::profiles::builtins()
        .into_iter()
        .find(|p| p.id == id)
        .unwrap()
}
// The shipped roster has no dedicated asset role; the asset-only catalog is kept with
// an inline profile so the atomic role-scoped loading boundary stays covered.
fn asset_profile() -> crate::assistant::profiles::AgentProfile {
    let mut p = profile("production");
    p.tool_ids = vec![
        "project-read".into(),
        "project-assets".into(),
        "media-generation".into(),
    ];
    p
}
fn catalog(profile: &crate::assistant::profiles::AgentProfile) -> Vec<ToolDefinition> {
    super::super::operation_tools::tools(profile)
        .into_iter()
        .map(|t| t.definition)
        .collect()
}
// Mirrors available_definitions(): the role's read tools, its deferred catalog and the
// framework tools the registry adds separately. Omitting any group would let the enum look
// correct while production exposes names this fixture never considers.
fn available(profile: &crate::assistant::profiles::AgentProfile) -> Vec<ToolDefinition> {
    let mut all = super::super::read_tools::definitions();
    all.extend(catalog(profile));
    let frame = |name: &str| ToolDefinition {
        name: name.into(),
        description: name.into(),
        parameters: json!({"type":"object","properties":{},"additionalProperties":false}),
    };
    for (tool, allowed) in [
        (
            "mstudio_models",
            crate::assistant::profiles::allows(profile, "models"),
        ),
        (
            "mstudio_await_generation",
            crate::assistant::profiles::allows(profile, "models"),
        ),
        ("mstudio_read_result", true),
    ] {
        if allowed {
            all.push(frame(tool));
        }
    }
    all
}
fn discovery_enum(definitions: &[ToolDefinition]) -> Vec<String> {
    definitions
        .iter()
        .find(|d| d.name == LOAD)
        .unwrap()
        .parameters["properties"]["names"]["items"]["enum"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
#[test]
fn discovery_enum_lists_every_name_the_role_may_select() {
    for profile in crate::assistant::profiles::builtins() {
        let all = available(&profile);
        let scope = LoadedTools::default();
        let definitions = scope.definitions(all.clone());
        for definition in &definitions {
            super::super::schema_definition::check(&definition.parameters).unwrap();
        }
        let selectable = discovery_enum(&definitions);
        // Every permitted tool is either loadable now or already exposed, so the enum is the
        // whole callable set: shrinking it to loaded tools alone would block all loading.
        for tool in &all {
            assert!(
                selectable.contains(&tool.name),
                "{} missing in {}",
                tool.name,
                profile.id
            );
        }
        // Neither the loader itself nor a name outside the role may be selected.
        assert!(!selectable.contains(&LOAD.to_owned()));
        assert!(
            super::super::scheduler::validate(
                &definitions,
                &rig_core::message::ToolCall::from_wire(
                    "load",
                    rig_core::message::ToolFunction {
                        name: LOAD.into(),
                        arguments: json!({"names":["mstudio_load_image"]}),
                    },
                ),
                &[]
            )
            .is_some()
        );
        let first = all[0].name.clone();
        scope.load(&all, &json!({"names":[first.clone()]}));
        assert!(discovery_enum(&scope.definitions(all)).contains(&first));
    }
}
#[test]
fn a_batch_mixing_an_exposed_tool_with_needed_tools_is_not_rejected() {
    // The real failure this guards: a subagent asked for five tools it needed plus
    // mstudio_read_generation, which it already had. Validation rejected the whole batch, so
    // none of the five were loaded and the run needed another step.
    let mut image = profile("image");
    image.tool_ids.push("media-generation".into());
    let scope = LoadedTools::default();
    let all = available(&image);
    let definitions = scope.definitions(all.clone());
    let requested = json!({"names":[
        "mstudio_generate_image", "mstudio_set_image_prompt", "mstudio_set_shot_frames",
        "mstudio_models", "mstudio_await_generation", "mstudio_read_generation"
    ]});
    let call = rig_core::message::ToolCall::from_wire(
        "load",
        rig_core::message::ToolFunction {
            name: LOAD.into(),
            arguments: requested.clone(),
        },
    );
    // The rejected name must be selectable, exactly as production exposes it.
    let selectable = discovery_enum(&definitions);
    for name in requested["names"].as_array().unwrap() {
        let name = name.as_str().unwrap();
        assert!(
            selectable.contains(&name.to_owned()),
            "{name} must be selectable"
        );
    }
    assert!(super::super::scheduler::validate(&definitions, &call, &[]).is_none());
    let value = scope.load(&all, &requested);
    assert_eq!(
        value["alreadyAvailable"],
        json!(["mstudio_read_generation"])
    );
    assert_eq!(value["loaded"].as_array().unwrap().len(), 5);
    // A name that is neither exposed nor permitted still fails.
    let bogus = rig_core::message::ToolCall::from_wire(
        "load",
        rig_core::message::ToolFunction {
            name: LOAD.into(),
            arguments: json!({"names":["mstudio_load_image"]}),
        },
    );
    assert!(super::super::scheduler::validate(&definitions, &bogus, &[]).is_some());
}
#[test]
fn video_agent_exposes_catalog_without_eager_edit_schemas() {
    let all = catalog(&profile("production"));
    let scope = LoadedTools::default();
    let before = scope.definitions(all.clone());
    assert_eq!(before.len(), 1);
    assert_eq!(before[0].name, LOAD);
    let original = serde_json::to_string(&all).unwrap().len();
    assert!(serde_json::to_string(&before).unwrap().len() < original / 2);
    let names = json!({"names":["mstudio_generate_video", "mstudio_update_generation"]});
    assert!(scope.load(&all, &names).get("error").is_none());
    let active = scope.definitions(all.clone());
    assert_eq!(active.len(), 3);
    assert!(!active.iter().any(|t| t.name == "mstudio_add_caption"));
    scope.load(&all, &names);
    assert_eq!(scope.definitions(all).len(), 3);
}
#[test]
fn loading_is_atomic_role_scoped_and_per_run() {
    let scope = LoadedTools::default();
    let all = catalog(&asset_profile());
    assert_eq!(
        scope.load(
            &all,
            &json!({"names":["mstudio_generate_reference_image", "mstudio_generate_video"]})
        )["code"],
        "FORBIDDEN"
    );
    assert_eq!(scope.definitions(all.clone()).len(), 1);
    scope.load(&all, &json!({"names":["mstudio_generate_reference_image"]}));
    assert_eq!(scope.definitions(all.clone()).len(), 2);
    assert_eq!(LoadedTools::default().definitions(all).len(), 1);
    assert_eq!(scope.definitions(catalog(&profile("writer"))).len(), 1);
    assert!(scope.definitions(catalog(&profile("concept"))).is_empty());
}
#[test]
fn deferred_names_drive_a_recoverable_rejection_then_disappear_once_loaded() {
    let scope = LoadedTools::default();
    let all = catalog(&profile("production"));
    let deferred = scope.deferred(&all);
    assert!(
        deferred
            .iter()
            .any(|name| name == "mstudio_set_video_prompt")
    );
    assert!(!deferred.iter().any(|name| name == LOAD));
    for base in BASE {
        assert!(!deferred.iter().any(|name| name == base), "{base}");
    }
    let call = rig_core::message::ToolCall::from_wire(
        "c",
        rig_core::message::ToolFunction {
            name: "mstudio_set_video_prompt".into(),
            arguments: json!({"id":"shot", "prompt":"Move"}),
        },
    );
    let rejected =
        super::super::scheduler::validate(&scope.definitions(all.clone()), &call, &deferred)
            .unwrap();
    assert_eq!(rejected["code"], "UNKNOWN_TOOL");
    assert!(
        rejected["error"]
            .as_str()
            .unwrap()
            .contains("mstudio_load_tools")
    );
    scope.load(&all, &json!({"names":["mstudio_set_video_prompt"]}));
    assert!(
        !scope
            .deferred(&all)
            .iter()
            .any(|n| n == "mstudio_set_video_prompt")
    );
}
#[test]
fn naming_an_always_available_tool_reports_it_instead_of_failing_the_batch() {
    // The operation catalog holds no base tools, matching what load() receives in production
    // for the name a model is most likely to name: base tools are judged before FORBIDDEN.
    let scope = LoadedTools::default();
    let all = catalog(&profile("production"));
    assert!(!all.iter().any(|t| t.name == "mstudio_read_project"));
    let value = scope.load(&all, &json!({"names":["mstudio_read_project"]}));
    assert_eq!(value["alreadyAvailable"], json!(["mstudio_read_project"]));
    assert!(value.get("error").is_none());
    // An unknown or unpermitted name still fails, so loading grants no permission.
    assert_eq!(
        scope.load(&all, &json!({"names":["mstudio_not_a_tool"]}))["code"],
        "FORBIDDEN"
    );
}
#[test]
fn scheduler_rejects_unloaded_tools_then_accepts_loaded_parameters() {
    let scope = LoadedTools::default();
    let all = catalog(&profile("production"));
    let call = rig_core::message::ToolCall::from_wire(
        "c",
        rig_core::message::ToolFunction {
            name: "mstudio_set_video_prompt".into(),
            arguments: json!({"id":"shot", "prompt":"Move"}),
        },
    );
    assert!(
        super::super::scheduler::validate(&scope.definitions(all.clone()), &call, &[]).is_some()
    );
    scope.load(&all, &json!({"names":["mstudio_set_video_prompt"]}));
    assert!(super::super::scheduler::validate(&scope.definitions(all), &call, &[]).is_none());
}
