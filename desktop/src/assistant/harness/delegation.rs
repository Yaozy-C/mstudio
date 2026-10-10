//! A bounded, non-recursive specialist run. Child events never pollute parent replay.
use super::{Host, ProjectHost, registry, session::Session};
use crate::assistant::{config::Profile, context, journal, profiles, provider, skills};
use crate::database::Store;
use rig_core::{
    completion::ToolDefinition,
    message::{Message, ToolCall},
};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
use tauri::Manager;
use tokio_util::sync::CancellationToken;
#[derive(Clone)]
pub struct Context {
    pub profile: Profile,
    pub key: String,
    pub model_id: Option<String>,
    pub fork_history: Vec<Message>,
    pub deadline: tokio::time::Instant,
}
pub fn definition() -> ToolDefinition {
    ToolDefinition { name:"mstudio_delegate".into(), description:"Delegate a self-contained task to a specialist: goal, object references, preserved requirements and expected result. Default spawn uses independent context; fork only when completed parent history is needed. Default mode=oneShot waits for the result and cannot continue after settlement. mode=continuable runs in the background by default, returns a childId for send_message, and notifies you when it settles. Set runInBackground=false when your next action depends on the result. Select continuable for expected follow-ups. Returns mode, canContinue, stop reason, answer and actual operation results; failure does not roll back saved work.".into(), parameters:json!({"type":"object","properties":{"agentId":{"type":"string"},"task":{"type":"string","minLength":1,"maxLength":8000},"provider":{"type":"string","enum":["spawn","fork"]},"modelId":{"type":"string"},"mode":{"type":"string","enum":["oneShot","continuable"]},"runInBackground":{"type":"boolean","description":"oneShot defaults to false; continuable defaults to true. Set false when your next action depends on the result."}},"required":["agentId","task"],"additionalProperties":false}) }
}

pub fn control_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {name:"mstudio_send_message".into(),description:"Send follow-up work to a direct continuable child. Running children read it at the next model step; idle children start a new turn. agentId is the returned childId, only with canContinue=true. For oneShot, delegate again with existing results. Returns only the accepted message ID.".into(),parameters:json!({"type":"object","properties":{"agentId":{"type":"string"},"message":{"type":"string","maxLength":8000}},"required":["agentId","message"],"additionalProperties":false})},
        ToolDefinition {name:"mstudio_interrupt_agent".into(),description:"Interrupt the current turn of a direct continuable child; it can still receive follow-up messages.".into(),parameters:json!({"type":"object","properties":{"agentId":{"type":"string"}},"required":["agentId"],"additionalProperties":false})},
        ToolDefinition {name:"mstudio_list_agents".into(),description:"List direct continuable children and their running, idle or resumable state. Only scope=children is supported; omission selects children.".into(),parameters:json!({"type":"object","properties":{"scope":{"type":"string","enum":["children"]}},"additionalProperties":false})},
    ]
}
struct ActiveRun {
    generation: String,
    token: CancellationToken,
}
static ACTIVE: OnceLock<Mutex<HashMap<String, ActiveRun>>> = OnceLock::new();
fn active() -> &'static Mutex<HashMap<String, ActiveRun>> {
    ACTIVE.get_or_init(|| Mutex::new(HashMap::new()))
}
fn remove_active(id: &str, generation: &str) {
    let mut active = active().lock().unwrap();
    if active
        .get(id)
        .is_some_and(|run| run.generation == generation)
    {
        active.remove(id);
    }
}
struct ChildHost {
    inner: ProjectHost,
    id: String,
    scripts: Mutex<Vec<Value>>,
    changes: Mutex<Vec<Value>>,
    terminal_reason: Mutex<Option<String>>,
}
impl Host for ChildHost {
    fn result_turn(&self) -> Option<&str> {
        self.inner.result_turn()
    }
    fn token(&self) -> &CancellationToken {
        &self.inner.token
    }
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.inner.definitions()
    }
    fn parallel_safe(&self, call: &ToolCall) -> bool {
        self.inner.parallel_safe(call)
    }
    fn injected(&self) -> Result<Vec<Message>, String> {
        let tool = self
            .inner
            .tool
            .as_ref()
            .ok_or("Subagent project context missing")?;
        super::mailbox::take_inbox(
            &tool.app.state::<Store>(),
            &self.id,
            &tool.project,
            &tool.turn,
        )
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        let tool = self
            .inner
            .tool
            .as_ref()
            .ok_or("Subagent project context missing")?;
        // Child replay stays isolated; public progress uses the same event path.
        tool.record(kind, value.clone())?;
        crate::assistant::child_activity::record(tool, kind, &value).map_err(|e| e.to_string())?;
        if kind == "model/stop" {
            *self.terminal_reason.lock().unwrap() = value["stopReason"].as_str().map(str::to_owned);
        }
        if kind == "tool/result" && value["value"]["applied"] == true {
            self.changes.lock().unwrap().push(
                json!({"callId":value["callId"],"name":value["name"],"result":value["value"]}),
            );
            if let Some(scripts) = value["value"]["scripts"].as_array() {
                self.scripts.lock().unwrap().extend(scripts.clone());
            }
        }
        Ok(())
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        if call.function.name == super::tool_loading::LOAD {
            return self.inner.loaded_tools.load(
                &self.inner.available_definitions(),
                &call.function.arguments,
            );
        }
        if call.function.name == "mstudio_read_image" {
            return self.inner.read_image(call).await;
        }
        registry::execute_local(self.inner.tool.as_ref().unwrap(), call).await
    }
}
mod continuation;
mod control;
mod execution;
mod spawn;
use continuation::{start_continuation, wake_pending};
#[cfg(test)]
use control::require_continuable;
pub use control::{execute_control, notices};
use execution::{drive_child, settle, stop_reason};
pub use spawn::execute;

fn scoped(
    mut profile: profiles::AgentProfile,
    parent: &str,
) -> Result<profiles::AgentProfile, String> {
    if !profile.enabled || profile.id == parent || profile.id == "coordinator" {
        return Err("This Agent cannot be delegated to".into());
    }
    profile.tool_ids.retain(|tool| tool != "agent-delegate");
    Ok(profile)
}
#[cfg(test)]
#[path = "delegation_tests.rs"]
mod tests;
