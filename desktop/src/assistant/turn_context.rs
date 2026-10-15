//! Shared role, project and model context for initial and resumed turns.
use super::{attachments::Reference, config::Profile, profiles::AgentProfile, tools::ProjectTool};
use crate::database::Store;
use rig_core::message::Message;
use serde_json::{Value, json};
use tauri::Manager;

pub fn snapshot(
    app: &tauri::AppHandle,
    role: &AgentProfile,
    doc: &Value,
    production: &Value,
    selected: Option<&str>,
) -> Result<Value, String> {
    let mut snapshot = if super::profiles::allows(role, "inspect") {
        super::context::project_snapshot(doc, selected)
    } else {
        json!({"projectId":doc["id"]})
    };
    snapshot["agent"] = super::model_profile::role(role);
    snapshot["skills"] = super::skills::runtime_catalog(app, &super::profiles::skill_setting(role))
        .map_err(|e| e.to_string())?;
    super::prompt_guidance::attach(
        &mut snapshot,
        &app.state::<Store>().db.lock().unwrap(),
        role,
        production,
        doc,
    )
    .map_err(|e| e.to_string())?;
    if role.id == "video-analyst" {
        super::video_analysis::isolate(&mut snapshot);
    }
    Ok(snapshot)
}

pub fn payload(
    store: &Store,
    doc: &Value,
    prompt: &str,
    refs: &[Reference],
    model: &Profile,
    role: &AgentProfile,
) -> Result<Value, String> {
    if role.id == "video-analyst" {
        super::video_analysis::validate_video(store, refs, model).map_err(|e| e.to_string())?;
    }
    super::attachments::payload(store, doc, prompt, refs, model).map_err(|e| e.to_string())
}

/// Refresh current facts and permissions without replaying old actions or user input.
pub fn refresh(tool: &mut ProjectTool, messages: &mut Vec<Message>) -> Result<(), String> {
    let store = tool.app.state::<Store>();
    tool.skill_setting = super::profiles::skill_setting(&tool.profile);
    let doc = crate::project_service::load(&store.db.lock().unwrap(), &tool.project)
        .map_err(|e| e.to_string())?;
    let original =
        super::generation_context::request(&store.db.lock().unwrap(), &tool.project, &tool.turn)
            .map_err(|e| e.to_string())?
            .map(|(_, request)| request)
            .unwrap_or(Value::Null);
    let snapshot = snapshot(
        &tool.app,
        &tool.profile,
        &doc,
        &original["production"],
        None,
    )?;
    *messages = super::harness::handoff::messages(super::prompts::system(&snapshot), messages);
    let snapshot =
        if let Some(scope) = super::task_context::saved(&store, &tool.project, &tool.turn)? {
            super::task_context::snapshot(snapshot, &scope, &doc)
        } else {
            snapshot
        };
    let snapshot = super::task_context::reference_snapshot(snapshot);
    let mut current = vec![super::harness::context_source::snapshot(snapshot)];
    super::harness::context_boundary::refresh_snapshot(messages, &mut current);
    messages.extend(current);
    Ok(())
}
