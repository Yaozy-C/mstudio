//! Select a compatible task session even after another role has run in the project.
use crate::{
    assistant::{journal, task_context},
    database::Store,
};
use rig_core::message::Message;
use rusqlite::params;
use serde_json::{Value, json};

/// Every user turn continues the task's durable message surface, including tools
/// and compaction. Explicit recovery selects the interrupted turn instead.
pub fn for_message(
    store: &Store,
    project: &str,
    binding: &Value,
    resume: Option<&str>,
) -> Result<Option<Vec<Message>>, String> {
    if let Some(turn) = resume {
        super::session::restore(store, project, turn, binding)
    } else {
        latest(store, project, binding)
    }
}

pub fn bind_task(
    store: &Store,
    project: &str,
    turn: &str,
    resume: Option<&str>,
    binding: &mut Value,
) -> Result<(), String> {
    let scope = task_context::saved(store, project, resume.unwrap_or(turn))?
        .ok_or("缺少任务上下文，请作为新任务发送")?;
    binding["taskId"] = json!(scope.task_id);
    let doc = crate::project_service::load(&store.db.lock().unwrap(), project)
        .map_err(|e| e.to_string())?;
    let archive = crate::assistant::archive_context::revision(&doc);
    if archive["archived"]
        .as_array()
        .is_some_and(|v| !v.is_empty())
    {
        binding["archiveContext"] = archive;
    }
    Ok(())
}

pub fn latest(
    store: &Store,
    project: &str,
    binding: &Value,
) -> Result<Option<Vec<Message>>, String> {
    let candidates = {
        let db = store.db.lock().unwrap();
        let mut stmt=db.prepare("SELECT e.turn_id,json_extract(e.payload,'$.binding'),json_extract(e.payload,'$.contextMode') FROM agent_events e WHERE e.project_id=?1 AND e.kind='session/start' AND json_extract(e.payload,'$.binding.agentId') IS ?2 AND json_extract(e.payload,'$.binding.taskId') IS ?3 AND e.seq>COALESCE((SELECT MAX(seq) FROM agent_events WHERE project_id=?1 AND kind='session/reset'),0) AND EXISTS(SELECT 1 FROM agent_messages m WHERE m.project_id=e.project_id AND m.role='assistant' AND json_extract(m.attribution,'$.turnId')=e.turn_id) ORDER BY e.seq DESC LIMIT 1").map_err(|e|e.to_string())?;
        stmt.query_map(
            params![
                project,
                binding["agentId"].as_str(),
                binding["taskId"].as_str()
            ],
            |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                ))
            },
        )
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };
    if let Some((turn, raw, mode)) = candidates.into_iter().next() {
        let saved: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        // Legacy snapshots deliberately discarded earlier exchanges. Seed once
        // from the task's chat log, then preserve the new continuous surface.
        // Archived-task projections also rebuild from that log and fresh facts.
        if mode.as_deref() != Some("continuous")
            || saved["archiveContext"] != binding["archiveContext"]
        {
            return Ok(None);
        }
        // Model selection is request configuration, not task identity. Preserve
        // canonical messages across a switch; the current adapter owns the wire.
        return super::session::restore(store, project, &turn, &saved);
    }
    Ok(None)
}

pub fn record_selection(
    store: &Store,
    project: &str,
    turn: &str,
    binding: &Value,
    count: usize,
) -> Result<(), String> {
    journal::append(
        store,
        project,
        turn,
        "context/selection",
        json!({"taskId":binding["taskId"],"agentId":binding["agentId"],"restoredMessages":count}),
    )
    .map_err(|e| e.to_string())
}
