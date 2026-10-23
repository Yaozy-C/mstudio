use super::{agent, attachments, context, history, journal, pending, profiles, task_target, tools};
use crate::database::Store;
use serde::Deserialize;
use serde_json::{Value, json};
use tauri::Manager;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    #[serde(default)]
    pub new_task: bool,
    #[serde(default)]
    pub resume_turn_id: Option<String>,
    pub prompt: String,
    pub attachments: Option<Vec<attachments::Reference>>,
    pub selected_node_id: Option<String>,
    pub task_node_id: Option<String>,
    pub selection: Option<Value>,
    pub model_id: Option<String>,
    pub agent_id: Option<String>,
    #[serde(default)]
    pub agent_revision: Option<u64>,
    pub client_turn_id: String,
    pub message_context: Value,
    #[serde(default, rename = "agentName")]
    pub agent_name: String,
    pub model_name: String,
}

impl Request {
    fn resolve_agent(&self, db: &rusqlite::Connection) -> anyhow::Result<profiles::AgentProfile> {
        // Client revision is diagnostic only. Each turn snapshots the latest saved profile.
        profiles::resolve(db, self.agent_id.as_deref())
    }
}

pub async fn run(
    app: tauri::AppHandle,
    request: Request,
) -> Result<super::execution::Reply, String> {
    if request.prompt.trim().is_empty() || request.prompt.chars().count() > 8000 {
        return Err("消息需为 1–8000 个字符".into());
    }
    let pending = pending::PendingTurn::begin(&request.project_id).map_err(|e| e.to_string())?;
    super::parent_activation::close(&request.project_id);
    let store = app.state::<Store>();
    if let Some(turn) = &request.resume_turn_id {
        super::harness::session::validate_resume(
            &store,
            &request.project_id,
            turn,
            &request.message_context,
        )?;
    }
    let agent_profile = request
        .resolve_agent(&store.db.lock().unwrap())
        .map_err(|e| e.to_string());
    // Persist even a profile rejection so a later retry keeps its original context.
    let current = agent_profile.as_ref().ok();
    let scope = super::task_context::resolve(
        &store,
        &request,
        current
            .map(|p| p.id.as_str())
            .or(request.agent_id.as_deref())
            .unwrap_or("coordinator"),
    )?;
    let meta = json!({"agentId":current.map(|p|p.id.as_str()).or(request.agent_id.as_deref()),"agentName":current.map(|p|p.name.as_str()).unwrap_or(&request.agent_name),
        "modelName":request.model_name,"createdAt":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).unwrap_or_default().as_millis() as u64,"turnId":request.client_turn_id,"request":request.message_context,"taskScope":scope});
    history::begin(
        &store,
        &request.project_id,
        &request.client_turn_id,
        &request.message_context,
        request.model_id.as_deref().unwrap_or(""),
        &meta,
    )
    .map_err(|e| e.to_string())?;
    super::execution::publish(&app, &request.project_id).map_err(|e| e.to_string())?;
    let result = match agent_profile {
        Ok(profile) => execute(&app, &request, &pending, profile, &scope).await,
        Err(error) => Err(error),
    };
    let status = super::execution::finish(
        &app,
        &request.project_id,
        &request.client_turn_id,
        &result,
        &pending.token,
    )
    .map_err(|e| e.to_string())?;
    drop(pending);
    super::parent_activation::settled(
        &request.project_id,
        &request.client_turn_id,
        result.is_ok(),
    )?;
    result.map(|text| super::execution::Reply { text, status })
}

async fn execute(
    app: &tauri::AppHandle,
    request: &Request,
    pending: &pending::PendingTurn,
    agent_profile: profiles::AgentProfile,
    scope: &super::task_context::Scope,
) -> Result<String, String> {
    let Request {
        project_id,
        prompt,
        attachments,
        selected_node_id,
        task_node_id,
        selection,
        model_id,
        agent_revision,
        client_turn_id,
        ..
    } = request;
    let store = app.state::<Store>();
    let (model, key) = crate::models::resolve(
        &store.db.lock().unwrap(),
        Some(project_id),
        model_id.as_deref(),
    )
    .map_err(|e| e.to_string())?;
    let profile = model.profile;
    let previous = super::task_context::history(&store, project_id, scope)?;
    let skill_setting = profiles::skill_setting(&agent_profile);
    let doc = crate::project_service::load(&store.db.lock().unwrap(), project_id)
        .map_err(|e| e.to_string())?;
    let refs = attachments.clone().unwrap_or_default();
    let analysis_only = agent_profile.id == "video-analyst";
    let mut payload =
        super::turn_context::payload(&store, &doc, prompt, &refs, &profile, &agent_profile)?;
    if !analysis_only {
        task_target::attach(&mut payload, &doc, task_node_id.as_deref())
            .map_err(|e| e.to_string())?;
    }
    let mut snapshot = super::turn_context::snapshot(
        app,
        &agent_profile,
        &doc,
        &request.message_context["production"],
        selected_node_id.as_deref(),
    )?;
    if let Some(scope) = selection
        .clone()
        .filter(|_| profiles::allows(&agent_profile, "inspect"))
    {
        let time = scope["time"]
            .as_f64()
            .filter(|t| t.is_finite() && *t >= 0. && *t <= 86400.)
            .unwrap_or(0.);
        let clip = doc["clips"]
            .as_array()
            .and_then(|v| v.iter().find(|c| c["id"] == scope["clipId"]));
        let source_time = clip.and_then(|c| {
            let local = time - c["start"].as_f64().unwrap_or(0.);
            let speed = c["speed"].as_f64()?;
            let duration = (c["trimOut"].as_f64()? - c["trimIn"].as_f64()?) / speed;
            (local >= 0. && local < duration).then(|| c["trimIn"].as_f64().unwrap() + local * speed)
        });
        snapshot["selection"] = json!({"time":time,"clip":clip,"sourceTime":source_time});
    }
    if !analysis_only {
        snapshot["workspace"] = super::work_context::resolve(
            &doc,
            &request.message_context["work"],
            task_node_id.as_deref(),
        )?;
    }
    snapshot = super::task_context::snapshot(snapshot, scope, &doc);
    let input = context::assemble_with_budget(
        &previous,
        payload.clone(),
        snapshot.clone(),
        profile.input_budget().unwrap_or(12_000).min(12_000),
    )?;
    let turn = client_turn_id;
    let record = |kind: &str, value: Value| {
        journal::append(&store, project_id, turn, kind, value).map_err(|e| e.to_string())
    };
    record(
        "turn/start",
        json!({"agentId":agent_profile.id,"agentRevision":agent_profile.revision,"requestedAgentRevision":agent_revision,"model":profile.model,"adapter":profile.adapter,"connectionId":model.id,"connectionName":model.name}),
    )?;
    record("user/message", json!({"text":prompt}))?;
    record(
        "request/context",
        json!({"project":snapshot,"attachments":payload[0]["attachments"],"messages":input.as_array().unwrap().iter().map(|m|json!({"role":m["role"],"content":context::text_only(&m["content"])})).collect::<Vec<_>>(),"budgetEstimatedTokens":profile.input_budget().unwrap_or(12_000).min(12_000),"taskId":scope.task_id}),
    )?;
    history::update_payload(&store, project_id, turn, &context::text_only(&payload))
        .map_err(|e| e.to_string())?;
    let tool = tools::ProjectTool {
        app: app.clone(),
        profile: agent_profile,
        skill_setting,
        project: project_id.clone(),
        turn: turn.clone(),
        prompt: prompt.clone(),
        token: pending.token.clone(),
        deadline: tokio::time::Instant::now() + std::time::Duration::from_secs(20 * 60),
    };
    let answer = agent::complete_with_resume(
        &profile,
        &key,
        input,
        Some(tool),
        request.resume_turn_id.as_deref(),
        Some(&model.id),
    )
    .await?;
    record("assistant/message", json!({"text":answer}))?;
    Ok(answer)
}

#[cfg(test)]
#[path = "chat_profile_tests.rs"]
mod profile_refresh_tests;
