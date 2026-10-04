use crate::assistant::{profiles, tools::ProjectTool};
use rig_core::completion::ToolDefinition;
use serde_json::{Value, json};
use tauri::Manager;

pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "mstudio_await_generation".into(),
        description: "Wait for already-created generation tasks when their results are needed for remaining authorized work. Pass task keys from receipts. This sleeps on backend events without polling the model or submitting work. Returns when any task has results, fails, is removed or needs user action; inspect its continuation and use resultAssetIds directly. waiting_user is returned immediately: report the required action, do not wait or inspect repeatedly. Do independent work before waiting. If the request only asked to submit, report the receipt without waiting.".into(),
        parameters: json!({"type":"object","properties":{"taskKeys":{"type":"array","minItems":1,"maxItems":30,"items":{"type":"string","minLength":1}}},"required":["taskKeys"],"additionalProperties":false}),
    }
}
pub async fn execute(tool: &ProjectTool, args: &Value) -> Value {
    if !profiles::allows(&tool.profile, "inspect") {
        return json!({"error":"Project read permission required","code":"FORBIDDEN"});
    }
    let keys: Vec<String> = args["taskKeys"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|key| key.as_str().map(str::to_owned))
        .collect();
    match crate::project_service::generation_wait::wait(
        &tool.app.state::<crate::database::Store>(),
        &tool.project,
        &keys,
        &tool.token,
        tool.deadline,
    )
    .await
    {
        Ok(value) => value,
        Err(error) => {
            json!({"error":error.to_string(),"code":"GENERATION_WAIT_FAILED","outcome":"not_executed"})
        }
    }
}
