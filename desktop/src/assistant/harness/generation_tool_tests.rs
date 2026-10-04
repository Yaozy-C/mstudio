use super::{generation_tool, tool_output};
use rig_core::message::{ToolCall, ToolFunction};
use serde_json::json;

#[test]
fn long_design_handoffs_keep_task_state_and_next_action_visible() {
    let task = json!({"id":"task","status":"AWAITING_CONFIRMATION","resultAssetIds":[],
        "continuation":{"state":"waiting_user","action":"confirm_generation","trigger":"user_confirmation"}});
    let call = ToolCall::from_wire(
        "child",
        ToolFunction {
            name: "mstudio_delegate".into(),
            arguments: json!({}),
        },
    );
    let result = json!({"ok":true,"stopReason":"completed","answer":"long rationale ".repeat(3000),
        "generationTasks":[task],"changes":[{"result":{"savedValues":[{"values":{"shot":{"framePrompt":"p".repeat(12000)}}}]}}]});
    let visible = tool_output::project(&call, &result, Some("turn"));
    assert_eq!(
        visible["generationTasks"][0]["status"],
        "AWAITING_CONFIRMATION"
    );
    assert_eq!(
        visible["generationTasks"][0]["continuation"],
        task["continuation"]
    );
    assert_eq!(visible["generationTasksComplete"], true);
    assert_eq!(visible["ok"], true);
    assert!(visible["detailOffloaded"].as_bool().unwrap());
}

#[test]
fn waiting_tool_has_a_closed_scoped_contract() {
    let definition = generation_tool::definition();
    assert!(
        super::schema::issues(&definition.parameters, &json!({"taskKeys":["task"]})).is_empty()
    );
    for invalid in [
        json!({"taskKeys":[]}),
        json!({"taskKeys":["task"],"projectId":"other"}),
        json!({"taskKeys":[42]}),
    ] {
        assert!(!super::schema::issues(&definition.parameters, &invalid).is_empty());
    }
}
