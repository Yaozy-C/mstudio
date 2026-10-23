//! A bounded, non-recursive specialist run. Child events never pollute parent replay.
use super::{Host, ProjectHost, registry, session::Session};
use crate::assistant::{config::Profile, journal, profiles, provider};
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
}
pub fn definition() -> ToolDefinition {
    ToolDefinition {
        name: "mstudio_delegate".try_into().expect("nonempty tool name"),
        description: "Start an independently managed specialist task and immediately return childId and outputRef with status=running. Completion arrives through a later notice with saved work, delivery and generationTasks. Continue independent work; do not redelegate running work. Child completion is not media completion.".into(),
        parameters: json!({"type":"object","properties":{
            "agentId":{"type":"string","description":"Specialist role ID."},
            "task":{"type":"string","minLength":1,"maxLength":8000,"description":"Brief goal, assigned targets, boundaries and deliverable. For separate shot outputs with existing inputs, assign one shot per call; share reference artifacts and continuity constraints across children. Sequence only new-result dependencies or overlapping writes. The host forwards the original user request and attachments; use artifacts for saved sources instead of restating them. For spawn, include needed decisions absent from those sources; for fork, state only new requirements."},
            "artifacts":super::handoff_artifacts::schema(),
            "expectedGenerations":super::delivery::schema(),
            "provider":{"type":"string","enum":["spawn","fork"],"description":"spawn (default): independent context. fork: inherits completed parent turns, excluding the current in-flight turn."},
            "modelId":{"type":"string","description":"Optional chat model connection ID for spawn; omission inherits the parent's chat model. Does not select the image or video model."},
            "mode":{"type":"string","enum":["oneShot","continuable"],"description":"oneShot (default) ends after this task. continuable supports later send_message calls using the returned childId. Both return immediately; completion is delivered later."}
        },"required":["agentId","task","expectedGenerations"],"additionalProperties":false}),
    }
}
pub(super) fn for_profiles(parent: &str, catalog: &[profiles::AgentProfile]) -> ToolDefinition {
    let mut tool = definition();
    let available: Vec<_> = catalog
        .iter()
        .filter(|profile| profile.enabled && profile.id != parent && profile.id != "coordinator")
        .collect();
    tool.parameters["properties"]["agentId"]["enum"] =
        json!(available.iter().map(|p| &p.id).collect::<Vec<_>>());
    let roles: Vec<_> = available
        .into_iter()
        .map(crate::assistant::model_profile::summary)
        .collect();
    tool.parameters["properties"]["agentId"]["description"] = json!(format!(
        "Specialist role. Available roles and permissions: {}",
        json!(roles)
    ));
    tool
}

pub fn control_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {name:"mstudio_send_message".try_into().expect("nonempty tool name"),description:"Send follow-up work to a direct continuable child. Running children receive it at the next step; idle children resume. Returns an accepted message ID, with completion delivered later.".into(),parameters:json!({"type":"object","properties":{"agentId":{"type":"string","description":"Returned childId of a child with canContinue=true."},"message":{"type":"string","maxLength":8000,"description":"New scope or correction. The host forwards current original user requirements; reference saved sources in artifacts."},"artifacts":super::handoff_artifacts::schema(),"expectedGenerations":super::delivery::schema()},"required":["agentId","message","expectedGenerations"],"additionalProperties":false})},
        ToolDefinition {name:"mstudio_interrupt_agent".try_into().expect("nonempty tool name"),description:"Interrupt the current turn of a direct continuable child; it can still receive follow-up messages.".into(),parameters:json!({"type":"object","properties":{"agentId":{"type":"string"}},"required":["agentId"],"additionalProperties":false})},
        ToolDefinition {name:"mstudio_list_agents".try_into().expect("nonempty tool name"),description:"List direct continuable children and their running, idle or resumable state. Only scope=children is supported; omission selects children.".into(),parameters:json!({"type":"object","properties":{"scope":{"type":"string","enum":["children"]}},"additionalProperties":false})},
    ]
}
struct ActiveRun {
    project: String,
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
    parent_turn: String,
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
    fn definitions(&self) -> Result<Vec<ToolDefinition>, String> {
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
            &self.inner.media_profile,
            &tool.profile,
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
        }
        Ok(())
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        if call.function.name == "mstudio_read_image" {
            return self.inner.read_image(call).await;
        }
        registry::execute_local(
            self.inner.tool.as_ref().unwrap(),
            call,
            &self.inner.media_profile,
        )
        .await
    }
}
mod continuation;
mod control;
mod execution;
mod persistence;
mod spawn;
pub fn cancel_project(project: &str) {
    for run in active()
        .lock()
        .unwrap()
        .values()
        .filter(|r| r.project == project)
    {
        run.token.cancel();
    }
}
use continuation::{start_continuation, wake_pending};
#[cfg(test)]
use control::require_continuable;
pub use control::{execute_control, notices};
use execution::drive_child;
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
