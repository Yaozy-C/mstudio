use super::*;
#[test]
fn both_child_modes_wait_unless_background_is_explicit() {
    for mode in ["oneShot", "continuable"] {
        assert!(!background(&json!({"mode":mode})));
        assert!(!background(&json!({"mode":mode,"runInBackground":false})));
        assert!(background(&json!({"mode":mode,"runInBackground":true})));
    }
}

#[test]
fn role_discovery_is_local_to_agent_id_and_preserves_custom_capabilities() {
    let mut catalog = profiles::builtins();
    let image = catalog.iter_mut().find(|p| p.id == "image").unwrap();
    image.name = "Custom image role".into();
    image.description = "Custom visual remit".into();
    image.tool_ids = vec!["project-read".into()];
    catalog
        .iter_mut()
        .find(|p| p.id == "writer")
        .unwrap()
        .enabled = false;
    let tool = for_profiles("director", &catalog);
    let parameter = &tool.parameters["properties"]["agentId"];
    let ids = parameter["enum"].as_array().unwrap();
    assert!(ids.contains(&json!("image")));
    assert!(
        !ids.iter()
            .any(|id| matches!(id.as_str(), Some("coordinator" | "director" | "writer")))
    );
    let roles = parameter["description"].as_str().unwrap();
    assert!(roles.contains("Custom visual remit"));
    assert!(roles.contains("Custom image role"));
    assert!(!tool.description.contains("Custom visual remit"));
    assert_eq!(
        serde_json::to_string(&tool)
            .unwrap()
            .matches("Custom visual remit")
            .count(),
        1
    );
    assert!(
        tool.parameters["properties"]["task"]["description"]
            .as_str()
            .unwrap()
            .contains("only new requirements")
    );
}

#[test]
fn specialist_keeps_own_permissions_and_cannot_recursively_delegate() {
    let mut writer = profiles::builtins()
        .into_iter()
        .find(|p| p.id == "writer")
        .unwrap();
    writer.tool_ids.push("agent-delegate".into());
    let child = scoped(writer.clone(), "coordinator").unwrap();
    assert!(!profiles::allows(&child, "delegate"));
    assert!(crate::assistant::permissions::allows_operation(
        &child,
        "update_node"
    ));
    assert!(!crate::assistant::permissions::allows_operation(
        &child,
        "request_generation"
    ));
    assert!(scoped(writer.clone(), "writer").is_err());
    writer.enabled = false;
    assert!(scoped(writer, "coordinator").is_err());
}

#[test]
fn base_exposure_never_substitutes_for_the_delegate_permission() {
    // mstudio_delegate is directly exposed, so the guard that keeps it away from other roles is
    // the profile permission alone; registry.rs adds the definition only under this predicate.
    for profile in profiles::builtins() {
        assert_eq!(
            profiles::allows(&profile, "delegate"),
            profile.id == "coordinator",
            "{} delegate permission",
            profile.id
        );
    }
    // A specialist that is handed the permission still loses it when it becomes a child.
    let mut writer = profiles::builtins()
        .into_iter()
        .find(|p| p.id == "writer")
        .unwrap();
    writer.tool_ids.push("agent-delegate".into());
    assert!(!profiles::allows(
        &scoped(writer, "coordinator").unwrap(),
        "delegate"
    ));
}
#[test]
fn continuation_reports_one_shot_and_unknown_children_distinctly() {
    assert!(require_continuable(Some("continuable")).is_ok());
    assert!(
        require_continuable(Some("oneShot"))
            .unwrap_err()
            .contains("oneShot")
    );
    assert!(
        require_continuable(None)
            .unwrap_err()
            .contains("mstudio_list_agents")
    );
}

#[test]
fn explicit_stop_cancels_detached_children_only_in_that_project() {
    let id = format!("cancel-test-{}", mstudio::media::id());
    let other = format!("{id}-other");
    let token = CancellationToken::new();
    let unrelated = CancellationToken::new();
    for (key, project, token) in [(&id, &id, &token), (&other, &other, &unrelated)] {
        active().lock().unwrap().insert(
            key.clone(),
            ActiveRun {
                project: project.clone(),
                generation: "turn".into(),
                token: token.clone(),
            },
        );
    }
    cancel_project(&id);
    assert!(token.is_cancelled());
    assert!(!unrelated.is_cancelled());
    remove_active(&id, "turn");
    remove_active(&other, "turn");
}
