use super::{history, journal, skills};
use crate::database::Store;
use serde_json::{Value, json};
use tauri::Manager;
use tokio_util::sync::CancellationToken;
#[derive(Clone)]
pub struct ProjectTool {
    pub app: tauri::AppHandle,
    pub profile: super::profiles::AgentProfile,
    pub skill_setting: String,
    pub project: String,
    pub turn: String,
    pub prompt: String,
    pub token: CancellationToken,
    pub deadline: tokio::time::Instant,
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
            crate::project_service::tool(
                self.app.clone(),
                self.profile.clone(),
                self.project.clone(),
                self.turn.clone(),
                id,
                args,
                self.token.clone(),
            )
            .await
        };
        if let Some(object) = result.as_object_mut() {
            object.remove("state");
        }
        result
    }
}
