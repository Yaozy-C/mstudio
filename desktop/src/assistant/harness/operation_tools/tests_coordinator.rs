//! Coordinator-level structure and task-edit coverage, split out to keep
//! each maintained test module within the source limit.
use super::tests::{decode, profile};
use serde_json::json;

#[test]
fn add_shot_reports_required_structure_and_coordinator_can_edit_tasks_directly() {
    let coordinator = profile("coordinator");
    let failed = decode(
        &coordinator,
        "mstudio_add_shot",
        json!({
            "id":"shot", "title":"Design", "text":"Short proposal", "duration":15
        }),
    )
    .unwrap()
    .unwrap_err();
    let paths: Vec<_> = failed["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| issue["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"$.screenplayId"));
    assert!(paths.contains(&"$.order"));
    assert_eq!(failed["outcome"], "not_executed");
    let update = decode(
        &coordinator,
        "mstudio_update_generation",
        json!({
            "taskKey":"existing-task", "prompt":"Complete redesigned prompt"
        }),
    )
    .unwrap()
    .unwrap();
    assert_eq!(update["operations"][0]["op"], "update_generation");
    assert_eq!(update["operations"][0]["taskKey"], "existing-task");
}
