mod agent;
mod chat;
pub(crate) mod child_activity;
mod codex_input;
mod codex_provider;
mod failure;
pub(crate) mod generation_context;
pub(crate) mod harness;
mod media_input;
pub mod media_prompt;
pub(crate) mod media_tools;
pub mod memory;
mod model_feedback;
mod model_profile;
pub(crate) mod permissions;
mod profile_instructions;
pub mod profiles;
mod progress;
mod prompt_guidance;
mod prompts;
mod provider;
mod task_context;
mod task_target;
mod work_context;
#[cfg(test)]
pub(crate) use agent::complete as complete_for_test;
mod attachments;
pub mod config;
pub(crate) mod context;
mod creative_context;
pub mod history;
pub mod journal;
pub mod pending;
pub mod skills;
pub(crate) mod tool_schema;
#[cfg(test)]
mod tool_schema_tests;
pub mod tools;
use crate::database::Store;
use anyhow::Result;
#[cfg(test)]
use serde_json::{Value, json};
use tauri::State;
#[tauri::command]
pub fn agent_history(
    store: State<Store>,
    project_id: String,
) -> Result<Vec<history::Message>, String> {
    history::read(&store, &project_id).map_err(|e| e.to_string())
}
#[cfg(test)]
pub fn messages(previous: &[history::Message], payload: Value) -> Value {
    context::assemble(previous, payload, json!({})).unwrap()
}
#[tauri::command]
pub async fn assistant_chat(
    app: tauri::AppHandle,
    request: chat::Request,
) -> Result<String, String> {
    chat::run(app, request).await
}

#[cfg(test)]
mod attachment_tests;
#[cfg(test)]
mod context_tests;
#[cfg(test)]
mod skill_tests;
#[cfg(test)]
mod tests;

#[cfg(test)]
mod multimodal_tests;

#[cfg(test)]
mod history_tests;

#[cfg(test)]
mod storyboard_tests;

#[cfg(test)]
mod bundled_skill_tests;
