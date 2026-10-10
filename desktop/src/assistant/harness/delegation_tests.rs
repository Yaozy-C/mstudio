use super::*;
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
    assert!(super::super::tool_loading::BASE.contains(&"mstudio_delegate"));
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
