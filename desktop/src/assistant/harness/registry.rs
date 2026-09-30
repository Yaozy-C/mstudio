use super::Host;
use crate::assistant::{memory::MemoryTool, profiles, tool_schema, tools::ProjectTool};
use rig_agent::{prelude::Tool, tool::ToolContext};
use rig_core::{completion::ToolDefinition, message::ToolCall};
use serde_json::{Value, json};
use tauri::Manager;
use tokio_util::sync::CancellationToken;

pub struct ProjectHost {
    pub tool: Option<ProjectTool>,
    pub media_profile: crate::assistant::config::Profile,
    pub token: CancellationToken,
    pub delegation: Option<super::delegation::Context>,
}
const ACTIONS: &[(&str, &str, &[&str])] = &[
    (
        "inspect",
        "Read project content needed for this task. For generation, query section=generation/taskKey or filter turnId/status; fields=[status,targetNodeId,resultAssetIds,error,trackingPaused] yields up to 30 tasks per page and whole-batch statistics. Follow nextOffset. For shots use nodeIds/fields; for scripts use fields=[script], paragraphIds and scriptFields. Omitted fields return summaries. Other lists have up to 12 entries per page and size limits; read small groups together with needed fields. Batch independent reads. revision is returned, not a query argument; establish current target state before editing.",
        &[
            "section",
            "ids",
            "taskKey",
            "turnId",
            "status",
            "nodeIds",
            "fields",
            "paragraphIds",
            "scriptFields",
            "offset",
            "textOffset",
        ],
    ),
    (
        "read_skill",
        "Read an enabled Skill or permitted reference. Batch independent files; follow nextOffset pages sequentially.",
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
        "Paginate enabled media models; use mediaModelId for model-specific prompt rules.",
        &["offset", "mediaModelId"],
    ),
    ("edit", "", &["revision", "operations"]),
];

impl ProjectHost {
    pub async fn read_image(&self, call: &ToolCall) -> Value {
        let Some(t) = &self.tool else {
            return json!({"error":"Tool unavailable"});
        };
        if !profiles::allows(&t.profile, "inspect") {
            return json!({"error":"Project read permission required","code":"FORBIDDEN"});
        }
        let store = t.app.state::<crate::database::Store>();
        let _files = store.files.read().await;
        let app = t.app.clone();
        let project = t.project.clone();
        let profile = self.media_profile.clone();
        let args = call.function.arguments.clone();
        tokio::task::spawn_blocking(move || {
            super::image_read::read(
                &app.state::<crate::database::Store>(),
                &project,
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
impl Host for ProjectHost {
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
        let Some(t) = &self.tool else { return vec![] };
        let schema = tool_schema::for_profile(&t.profile);
        let mut definitions = Vec::new();
        definitions.push(ToolDefinition { name:"mstudio_read_result".into(), description:"Read an offloaded complete tool result by page. callId comes from resultRef. Omitted turnId means this turn; supply the original turnId for history. Continue with returned nextOffset.".into(), parameters:json!({"type":"object","properties":{"callId":{"type":"string"},"turnId":{"type":"string"},"offset":{"type":"integer","minimum":0}},"required":["callId"],"additionalProperties":false}) });
        for (action, description, fields) in ACTIONS {
            if !profiles::allows(&t.profile, action) {
                continue;
            }
            let properties: serde_json::Map<_, _> = fields
                .iter()
                .map(|field| ((*field).to_owned(), schema["properties"][*field].clone()))
                .collect();
            let required = match *action {
                "edit" => vec!["revision", "operations"],
                "read_skill" => vec!["skill"],
                _ => vec![],
            };
            definitions.push(ToolDefinition {
                name: format!("mstudio_{action}"),
                description: if *action == "edit" { edit_description() } else { (*description).into() },
                parameters: json!({"type":"object","properties":properties,"required":required,"additionalProperties":false}),
            });
        }
        if profiles::allows(&t.profile, "memory-read") {
            let memory = MemoryTool(t.clone());
            definitions.push(ToolDefinition {
                name: "mstudio_memory".into(),
                description: memory.description(),
                parameters: memory.parameters(),
            });
        }
        if profiles::allows(&t.profile, "inspect") {
            definitions.push(ToolDefinition {
                name: "mstudio_read_image".into(),
                description: "Read real image pixels or a video frame directly into the model. Use an assetId returned by inspect. For video, time is source seconds unless clipId is supplied; then it is clip-local seconds, accounting for trim/speed and current grading, excluding transitions, overlays and captions. transition:true with the incoming clipId uses seconds from transition start and returns the actual transition composite, excluding overlays, captions and audio. Sample relevant beginnings, middles, endings and both sides of joins; samples do not prove full playback. Requires image-input support.".into(),
                parameters: json!({"type":"object","properties":{"assetId":{"type":"string"},"clipId":{"type":"string"},"transition":{"type":"boolean"},"time":{"type":"number","minimum":0}},"required":["assetId"],"additionalProperties":false}),
            });
            definitions.push(ToolDefinition {
                name: "mstudio_reopen_image".into(),
                description: "Reopen a historical offloaded image by imageId from its notice. Only offloaded images in this project are accessible; returns actual image content.".into(),
                parameters: json!({"type":"object","properties":{"imageId":{"type":"string"}},"required":["imageId"],"additionalProperties":false}),
            });
        }
        if profiles::allows(&t.profile, "delegate") {
            let mut definition = super::delegation::definition();
            let catalog =
                profiles::read(&t.app.state::<crate::database::Store>().db.lock().unwrap())
                    .unwrap_or_default();
            let ids: Vec<_> = catalog
                .iter()
                .filter(|p| p.enabled && p.id != t.profile.id && p.id != "coordinator")
                .map(|p| p.id.clone())
                .collect();
            definition.parameters["properties"]["agentId"]["enum"] = json!(ids);
            let roster: Vec<_> = catalog
                .iter()
                .filter(|p| ids.contains(&p.id))
                .map(|p| json!({"id":p.id,"tools":p.tool_ids}))
                .collect();
            definition.description.push_str(&format!("Available roles and permissions: {}. Shared white-background assets require project-assets; scripts project-script; shots project-shots; image prompts/images project-frames; video prompts/videos project-production; timeline project-timeline. Generation additionally requires media-generation.",json!(roster)));

            definitions.push(definition);
            definitions.extend(super::delegation::control_definitions());
        }
        definitions
    }
    fn parallel_safe(&self, call: &ToolCall) -> bool {
        // Project reads go through the UI edit queue and are ordering barriers.
        // Skill files/history/catalog are independent reads; memory mutations are exclusive.
        matches!(
            self.action(&call.function.name),
            Some("read_skill" | "skills" | "history" | "models")
        ) || (call.function.name == "mstudio_memory" && call.function.arguments["action"] == "list")
            || matches!(
                call.function.name.as_str(),
                "mstudio_reopen_image" | "mstudio_read_result"
            )
            || matches!(
                call.function.name.as_str(),
                "mstudio_send_message" | "mstudio_interrupt_agent" | "mstudio_list_agents"
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
        execute_local(t, call, "").await
    }
}

fn edit_description() -> String {
    "Batch authorized project edits using the latest inspect revision. Operations save atomically and support undo. Preserve existing IDs and omitted fields; add_node requires id/kind/title, and shots link screenplayId. update_node: screenplay.script holds paragraphs, node text holds shot action/staging, shot.prompt video drafts, shot.framePrompt/frames image drafts/assets. update_generation(taskKey,text) updates the task prompt, preserves existing results and does not change a shot draft or already-submitted request. Prompt edits do not generate; explicitly requested regenerate_generation(taskKey) reuses the model, inputs and parameters. request_generation requires the complete final prompt in text; scripts are not appended automatically. For shot references, use set_references with referenceMode=upsert to add/update by assetId, or referenceMode=remove with assetIds to unlink; reserve replace for an explicit full reset. Keep reference assets in project media, not new canvas nodes. references contains real asset IDs, purposes and roles; omission inherits only current composer references, not arbitrary project assets. Use the selected model. choose_take selects media; assemble_screenplay arranges shots. start is output time, trimIn/trimOut source range, speed absolute. savedClips contains actual saved timeline values: when complete=true, use it for parameter verification; inspect missing or uncertain values when complete=false. Receipts do not replace pixel/playback checks. Correct reported errors; inspect uncertain effects before retrying. Use assigned Skills for creative methods.".into()
}

pub async fn execute_local(t: &ProjectTool, call: &ToolCall, prefix: &str) -> Value {
    let mut args = call.function.arguments.clone();
    if call.function.name == "mstudio_reopen_image" {
        if !profiles::allows(&t.profile, "inspect") {
            return json!({"error":"Project read permission required","code":"FORBIDDEN"});
        }
        let Some(id) = args["imageId"]
            .as_str()
            .filter(|id| id.starts_with("image-") && id.len() < 100)
        else {
            return json!({"error":"This tool accepts only imageId from an offload notice. For generated images use mstudio_read_image(assetId), with a real asset ID from inspect, not a filename or task ID.","code":"INVALID_ARGS"});
        };
        let store = t.app.state::<crate::database::Store>();
        let db = store.db.lock().unwrap();
        return super::stored_image::read(&db, &t.project, id);
    }
    if call.function.name == "mstudio_read_result" {
        return super::tool_output::read(t, &args);
    }
    if call.function.name == "mstudio_memory" {
        return MemoryTool(t.clone())
            .call(&mut ToolContext::new(), args)
            .await
            .unwrap();
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
    t.execute(args, format!("{}:{}{}", t.turn, prefix, call.id.as_str()))
        .await
}

#[cfg(test)]
#[path = "registry_tests.rs"]
mod tests;
