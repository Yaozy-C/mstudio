use super::*;
#[test]
fn specialist_keeps_own_permissions_and_cannot_recursively_delegate() {
    let mut concept = profiles::builtins()
        .into_iter()
        .find(|p| p.id == "concept")
        .unwrap();
    concept.tool_ids.push("agent-delegate".into());
    let child = scoped(concept.clone(), "coordinator").unwrap();
    assert!(!profiles::allows(&child, "delegate"));
    assert!(crate::assistant::permissions::allows_operation(
        &child,
        "update_node"
    ));
    assert!(!crate::assistant::permissions::allows_operation(
        &child,
        "request_generation"
    ));
    assert!(scoped(concept.clone(), "concept").is_err());
    concept.enabled = false;
    assert!(scoped(concept, "coordinator").is_err());
}
