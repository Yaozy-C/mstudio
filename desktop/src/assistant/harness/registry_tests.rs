use super::*;
#[test]
fn project_discovery_has_no_umbrella_read_or_edit_actions() {
    assert!(!ACTIONS.iter().any(|(name, _, _)| *name == "inspect"));
    let definitions = super::super::read_tools::definitions();
    assert!(
        definitions
            .iter()
            .any(|tool| tool.name == "mstudio_read_shots")
    );
    assert!(
        !definitions
            .iter()
            .any(|tool| tool.name == "mstudio_inspect")
    );
}
