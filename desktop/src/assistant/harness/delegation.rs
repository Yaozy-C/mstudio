//! A bounded, non-recursive specialist run. Child events never pollute parent replay.
use super::{Host, ProjectHost, registry, session::Session};
use crate::assistant::{
    config::Profile,
    context, journal,
    memory::{MemoryBackend, SqliteMemory},
    profiles, provider, skills,
};
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
pub struct Context {
    pub profile: Profile,
    pub key: String,
    pub model_id: Option<String>,
    pub fork_history: Vec<Message>,
    pub deadline: tokio::time::Instant,
}
pub fn definition() -> ToolDefinition {
    ToolDefinition { name:"mstudio_delegate".into(), description:"向专业 Agent 委派明确任务。task 必须自包含：目标、对象引用、必须保留项及预期结果。默认 spawn 使用独立会话；需要父会话已完成历史时才用 fork。默认 mode=oneShot，结束后不可续接；预计需要修订或追问时显式使用 continuable。返回 mode、canContinue、结束原因、答复与真实操作结果；失败不代表已发生操作回滚。".into(), parameters:json!({"type":"object","properties":{"agentId":{"type":"string"},"task":{"type":"string","minLength":1,"maxLength":8000},"provider":{"type":"string","enum":["spawn","fork"]},"modelId":{"type":"string"},"mode":{"type":"string","enum":["oneShot","continuable"]},"runInBackground":{"type":"boolean"}},"required":["agentId","task"],"additionalProperties":false}) }
}

pub fn control_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {name:"mstudio_send_message".into(),description:"向直属可继续子 Agent 发送后续任务。运行中的子 Agent 在下一个模型步骤读取；空闲子 Agent 开始新一轮。agentId 必须用委派返回的 childId；仅当 canContinue=true 时可用，oneShot 应重新委派并附已有结果。只返回消息接收 ID。".into(),parameters:json!({"type":"object","properties":{"agentId":{"type":"string"},"message":{"type":"string","maxLength":8000}},"required":["agentId","message"],"additionalProperties":false})},
        ToolDefinition {name:"mstudio_interrupt_agent".into(),description:"停止直属子 Agent 的当前轮次；子 Agent 仍可继续接收后续消息。".into(),parameters:json!({"type":"object","properties":{"agentId":{"type":"string"}},"required":["agentId"],"additionalProperties":false})},
        ToolDefinition {name:"mstudio_list_agents".into(),description:"列出当前 Agent 的可继续子 Agent 及运行、空闲、可恢复状态。".into(),parameters:json!({"type":"object","properties":{"scope":{"type":"string","enum":["children","descendants"]}},"additionalProperties":false})},
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
        let tool = self.inner.tool.as_ref().ok_or("子 Agent 缺少项目上下文")?;
        super::mailbox::take_inbox(
            &tool.app.state::<Store>(),
            &self.id,
            &tool.project,
            &tool.turn,
        )
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        let tool = self.inner.tool.as_ref().ok_or("子 Agent 缺少项目上下文")?;
        // Persist first; only authoritative committed outcomes count as applied.
        journal::append(
            &tool.app.state::<Store>(),
            &tool.project,
            &tool.turn,
            kind,
            value.clone(),
        )
        .map_err(|e| e.to_string())?;
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
        if call.function.name == "mstudio_read_image" {
            return self.inner.read_image(call).await;
        }
        registry::execute_local(
            self.inner.tool.as_ref().unwrap(),
            call,
            &format!("{}:", self.id),
        )
        .await
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
        return Err("此 Agent 不可委派".into());
    }
    profile
        .tool_ids
        .retain(|tool| !["agent-delegate", "memory-write", "memory-read"].contains(&tool.as_str()));
    Ok(profile)
}
#[cfg(test)]
#[path = "delegation_tests.rs"]
mod tests;
