use super::Host;
use crate::assistant::{profiles, tool_schema, tools::ProjectTool};
use crate::database::Store;
use rig_core::{completion::ToolDefinition, message::ToolCall};
use serde_json::{Value, json};
use tauri::Manager;
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
pub struct ProjectHost {
    pub tool: Option<ProjectTool>,
    pub media_profile: crate::assistant::config::Profile,
    pub token: CancellationToken,
    pub delegation: Option<super::delegation::Context>,
}
const ACTIONS: &[(&str, &str, &[&str])] = &[
    (
        "read_skill",
        "Read the complete text of an enabled Skill or permitted reference. Reuse loaded text; read references for concrete missing information and batch independent reads.",
        &["skill", "path", "offset"],
    ),
    (
        "skills",
        "List Skills and resource directories enabled for this Agent.",
        &[],
    ),
    (
        "history",
        "Paginate project conversation history; taskId narrows to the current or known task. Follow textOffset for full text.",
        &["offset", "messageId", "textOffset", "taskId"],
    ),
    (
        "models",
        "Paginate enabled media models; use mediaModelId for model-specific prompt rules and parameter capabilities, including exact enums, bounds and configured defaults. Match the generation tool’s parameters to that model’s capability schema; do not copy project export resolution or editorial shot duration as generation settings.",
        &["offset", "mediaModelId"],
    ),
    ("edit", "", &["operations"]),
];

impl ProjectHost {
    pub async fn read_image(&self, call: &ToolCall) -> Value {
        let Some(t) = &self.tool else {
            return json!({"error":"Tool unavailable"});
        };
        if let Err(error) = super::media_tools::check(&t.profile) {
            return error;
        }
        let store = t.app.state::<crate::database::Store>();
        let _files = store.files.read().await;
        let app = t.app.clone();
        let project = t.project.clone();
        let turn = t.turn.clone();
        let profile = self.media_profile.clone();
        let args = serde_json::Value::Object(call.function.arguments.clone());
        tokio::task::spawn_blocking(move || {
            super::image_read::read(
                &app.state::<crate::database::Store>(),
                &project,
                &turn,
                &profile,
                &args,
            )
        })
        .await
        .unwrap_or_else(|e| json!({"error":format!("Media read failed: {e}")}))
    }

    fn action<'a>(&self, name: &'a str) -> Option<&'a str> {
        let action = name.strip_prefix("mstudio_")?;
        ACTIONS
            .iter()
            .any(|(a, _, _)| *a == action)
            .then_some(action)
    }
}
impl ProjectHost {
    pub(super) fn available_definitions(&self) -> Vec<ToolDefinition> {
        let Some(t) = &self.tool else { return vec![] };
        if t.profile.id == "video-analyst" {
            return vec![];
        }
        let schema = tool_schema::for_profile(&t.profile);
        let mut definitions = Vec::new();
        if profiles::allows(&t.profile, "inspect") {
            definitions.push(super::agent_artifacts::definition());
            definitions.push(super::generation_tool::definition());
            definitions.extend(super::read_tools::definitions());
        }
        definitions.push(ToolDefinition { name:"mstudio_read_result".try_into().expect("nonempty tool name"), description:"Read an offloaded complete tool result by page. callId comes from resultRef. Omitted turnId means this turn; supply the original turnId for history. Continue with returned nextOffset.".into(), parameters:json!({"type":"object","properties":{"callId":{"type":"string"},"turnId":{"type":"string"},"offset":{"type":"integer","minimum":0}},"required":["callId"],"additionalProperties":false}) });
        for (action, description, fields) in ACTIONS {
            if !profiles::allows(&t.profile, action) {
                continue;
            }
            if *action == "edit" {
                definitions.extend(
                    super::operation_tools::tools(&t.profile)
                        .into_iter()
                        .map(|tool| tool.definition),
                );
                continue;
            }
            let properties: serde_json::Map<_, _> = fields
                .iter()
                .map(|field| ((*field).to_owned(), schema["properties"][*field].clone()))
                .collect();
            let required = match *action {
                "edit" => vec!["operations"],
                "read_skill" => vec!["skill"],
                _ => vec![],
            };
            definitions.push(ToolDefinition {
                name: format!("mstudio_{action}").try_into().expect("nonempty tool name"),
                description: (*description).into(),
                parameters: json!({"type":"object","properties":properties,"required":required,"additionalProperties":false}),
            });
        }
        definitions.extend(super::media_tools::definitions(&t.profile));
        if profiles::allows(&t.profile, "delegate") {
            let catalog =
                profiles::read(&t.app.state::<crate::database::Store>().db.lock().unwrap())
                    .unwrap_or_default();
            definitions.push(super::delegation::for_profiles(&t.profile.id, &catalog));
            definitions.extend(super::delegation::control_definitions());
        }
        definitions
    }
}
impl Host for ProjectHost {
    fn pending_children(&self) -> bool {
        let Some(tool) = &self.tool else { return false };
        crate::assistant::execution::pending(
            &tool.app.state::<Store>().db.lock().unwrap(),
            &tool.project,
            &tool.turn,
        )
        .unwrap_or(false)
    }

    fn deadline(&self) -> Option<tokio::time::Instant> {
        self.tool.as_ref().map(|t| t.deadline)
    }
    fn result_turn(&self) -> Option<&str> {
        self.tool.as_ref().map(|t| t.turn.as_str())
    }
    fn token(&self) -> &CancellationToken {
        &self.token
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        match &self.tool {
            Some(t) => t.record(kind, value),
            None => Ok(()),
        }
    }
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.available_definitions()
    }
    fn parallel_safe(&self, call: &ToolCall) -> bool {
        // Project reads are barriers relative to edits; execution is native and transactional.
        // Skill files/history/catalog are independent reads.
        matches!(
            self.action(&call.function.name),
            Some("read_skill" | "skills" | "history" | "models")
        ) || matches!(
            call.function.name.as_str(),
            "mstudio_reopen_image"
                | "mstudio_reopen_media"
                | "mstudio_read_result"
                | "mstudio_read_agent_result"
        ) || matches!(
            call.function.name.as_str(),
            "mstudio_delegate"
                | "mstudio_send_message"
                | "mstudio_interrupt_agent"
                | "mstudio_list_agents"
        )
    }
    fn injected(&self) -> Result<Vec<rig_core::message::Message>, String> {
        self.tool
            .as_ref()
            .filter(|t| profiles::allows(&t.profile, "delegate"))
            .map_or(Ok(vec![]), super::delegation::notices)
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        let Some(t) = &self.tool else {
            return json!({"error":"Tool unavailable", "code":"UNKNOWN_TOOL"});
        };
        if t.profile.id == "video-analyst" {
            return json!({"error":"Video analysis has no tools", "code":"FORBIDDEN"});
        }
        if call.function.name == "mstudio_delegate" {
            return super::delegation::execute(self, call).await;
        }
        if matches!(
            call.function.name.as_str(),
            "mstudio_send_message" | "mstudio_interrupt_agent" | "mstudio_list_agents"
        ) {
            return super::delegation::execute_control(self, call).await;
        }
        if call.function.name == "mstudio_read_image" {
            return self.read_image(call).await;
        }
        execute_local(t, call, &self.media_profile).await
    }
}

pub async fn execute_local(
    t: &ProjectTool,
    call: &ToolCall,
    media_profile: &crate::assistant::config::Profile,
) -> Value {
    if t.profile.id == "video-analyst" {
        return json!({"error":"Video analysis has no tools", "code":"FORBIDDEN"});
    }
    let mut args = serde_json::Value::Object(call.function.arguments.clone());
    if let Some(decoded) = super::read_tools::decode(&call.function.name, args.clone()) {
        let value = t
            .execute(decoded, format!("{}:{}", t.turn, call.id.wire().as_ref()))
            .await;
        return super::read_tools::result(&call.function.name, value);
    }

    if let Some(decoded) =
        super::operation_tools::decode(&t.profile, &call.function.name, args.clone())
    {
        return t
            .execute(decoded, format!("{}:{}", t.turn, call.id.wire().as_ref()))
            .await;
    }

    if call.function.name == "mstudio_await_generation" {
        return super::generation_tool::execute(t, &args).await;
    }
    if call.function.name == "mstudio_reopen_media" {
        if let Err(error) = super::media_tools::check(&t.profile) {
            return error;
        }
        let Some(id) = args["mediaId"]
            .as_str()
            .filter(|id| super::media_retention::valid_reference(id, "media-"))
        else {
            return json!({"error":"Use a mediaId from an offload notice","code":"INVALID_ARGS"});
        };
        let store = t.app.state::<Store>();
        let db = store.db.lock().unwrap();
        return super::media_retention::read(&db, &t.project, id, media_profile);
    }
    if call.function.name == "mstudio_reopen_image" {
        if let Err(error) = super::media_tools::check(&t.profile) {
            return error;
        }
        let Some(id) = args["imageId"]
            .as_str()
            .filter(|id| super::media_retention::valid_reference(id, "image-"))
        else {
            return json!({"error":"This tool accepts only imageId from an offload notice. For generated images use mstudio_read_image(assetId), with a real asset ID from a receipt or inspected project state, not a filename or task ID.","code":"INVALID_ARGS"});
        };
        let store = t.app.state::<crate::database::Store>();
        let db = store.db.lock().unwrap();
        return super::stored_image::read(&db, &t.project, id);
    }
    if call.function.name == "mstudio_read_result" {
        return super::tool_output::read(t, &args);
    }
    if call.function.name == "mstudio_read_agent_result" {
        return super::agent_artifacts::read(t, &args);
    }
    let Some(action) = call
        .function
        .name
        .strip_prefix("mstudio_")
        .filter(|a| ACTIONS.iter().any(|(name, _, _)| name == a))
    else {
        return json!({"error":"Unknown tool", "code":"UNKNOWN_TOOL"});
    };
    args["action"] = json!(action);
    t.execute(args, format!("{}:{}", t.turn, call.id.wire().as_ref()))
        .await
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
