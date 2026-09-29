use super::{history, journal, skills};
use crate::database::Store;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
use tauri::{Emitter, Manager};
use tokio::sync::oneshot;
use tokio_util::sync::CancellationToken;
struct Reply {
    project: String,
    sender: oneshot::Sender<Value>,
    token: CancellationToken,
}
fn replies() -> &'static Mutex<HashMap<String, Reply>> {
    static VALUE: OnceLock<Mutex<HashMap<String, Reply>>> = OnceLock::new();
    VALUE.get_or_init(Default::default)
}
struct Cleanup(String);
impl Drop for Cleanup {
    fn drop(&mut self) {
        replies().lock().unwrap().remove(&self.0);
    }
}
#[tauri::command]
pub fn agent_tool_active(call_id: String, project_id: String) -> bool {
    replies()
        .lock()
        .unwrap()
        .get(&call_id)
        .is_some_and(|r| r.project == project_id && !r.token.is_cancelled())
}
#[tauri::command]
pub fn agent_tool_result(call_id: String, project_id: String, result: Value) -> Result<(), String> {
    if result.to_string().len() > 200_000 {
        return Err("Tool result too long".into());
    }
    let mut map = replies().lock().unwrap();
    if !map.get(&call_id).is_some_and(|r| r.project == project_id) {
        return Err("Tool request already settled".into());
    }
    if let Some(reply) = map.remove(&call_id) {
        let _ = reply.sender.send(result);
    }
    Ok(())
}
#[derive(Clone)]
pub struct ProjectTool {
    pub app: tauri::AppHandle,
    pub profile: super::profiles::AgentProfile,
    pub skill_setting: String,
    pub project: String,
    pub turn: String,
    pub prompt: String,
    pub token: CancellationToken,
}
impl ProjectTool {
    pub async fn execute(&self, args: Value, id: String) -> Value {
        let store = self.app.state::<Store>();
        if self.token.is_cancelled() {
            return json!({"error":"Task stopped"});
        }
        if !super::profiles::allows(&self.profile, args["action"].as_str().unwrap_or("")) {
            return json!({"error":"Required tool or Skill is not assigned to this Agent"});
        }
        if args["action"] == "edit" {
            let checked = (|| -> anyhow::Result<()> {
                let raw: String = store.db.lock().unwrap().query_row(
                    "SELECT document FROM projects WHERE id=?1",
                    [&self.project],
                    |r| r.get(0),
                )?;
                super::media_tools::validate_selection(&store, &args)?;
                super::permissions::validate(&self.profile, &args, &serde_json::from_str(&raw)?)
            })();
            if let Err(e) = checked {
                return json!({"error":e.to_string()});
            }
        }
        if args.to_string().len() > 24000 {
            return json!({"error":"Tool arguments exceed 24 KB; split the operations"});
        }
        let mut result = if args["action"] == "skills" || args["action"] == "read_skill" {
            match skills::tool(&self.app, &self.skill_setting, &args) {
                Ok(v) => {
                    let _ = journal::append(
                        &store,
                        &self.project,
                        &self.turn,
                        "skill/read",
                        json!({"skill":args["skill"],"path":args["path"],"offset":args["offset"]}),
                    );
                    v
                }
                Err(e) => json!({"error":e.to_string()}),
            }
        } else if args["action"] == "models" {
            match super::media_tools::catalog(&store, &args) {
                Ok(value) => value,
                Err(e) => json!({"error":e.to_string()}),
            }
        } else if args["action"] == "history" {
            match history::page(
                &store,
                &self.project,
                args["offset"].as_u64().unwrap_or(0).min(100_000) as usize,
                args["messageId"].as_i64(),
                args["textOffset"].as_u64().unwrap_or(0).min(100_000) as usize,
                args["taskId"].as_str(),
            ) {
                Ok(v) => json!(v),
                Err(e) => json!({"error":e.to_string()}),
            }
        } else {
            let generation =
                match super::generation_context::production(&store, &self.project, &self.turn) {
                    Ok(value) => value,
                    Err(error) => return json!({"error":error.to_string()}),
                };
            let (sender, receiver) = oneshot::channel();
            replies().lock().unwrap().insert(
                id.clone(),
                Reply {
                    project: self.project.clone(),
                    sender,
                    token: self.token.clone(),
                },
            );
            let _cleanup = Cleanup(id.clone());
            if self
                .app
                .emit(
                    "agent-project-tool",
                    json!({"callId":id,"projectId":self.project,"agentId":self.profile.id,"turnId":self.turn,"generation":generation,"args":args}),
                )
                .is_err()
            {
                json!({"error":"Cannot connect to the project UI"})
            } else {
                // Once dispatched, drain the UI acknowledgement even after cancellation.
                // A timeout is an unknown effect, never evidence that the edit did not happen.
                match tokio::time::timeout(std::time::Duration::from_secs(30), receiver).await {
                    Ok(Ok(value)) => value,
                    _ => json!({"error":"Project operation unconfirmed; inspect current state before deciding the next action", "code":"EFFECT_UNKNOWN"}),
                }
            }
        };
        if let Some(object) = result.as_object_mut() {
            object.remove("state");
        }
        result
    }
}
