mod agent;
mod chat;
mod failure;
mod generation_context;
mod harness;
mod media_input;
pub mod media_prompt;
mod media_tools;
pub mod memory;
mod permissions;
pub mod profiles;
mod progress;
mod prompts;
mod provider;
mod task_context;
mod task_target;
mod work_context;
#[cfg(test)]
pub(crate) use agent::complete as complete_for_test;
mod attachments;
pub mod config;
mod context;
mod creative_context;
pub mod history;
pub mod journal;
pub mod pending;
pub mod skills;
mod tool_schema;
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
