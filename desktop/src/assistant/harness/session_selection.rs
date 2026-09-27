//! Select a compatible task session even after another role has run in the project.
use crate::{
    assistant::{journal, task_context},
    database::Store,
};
use rig_core::message::Message;
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};

pub fn bind_task(
    store: &Store,
    project: &str,
    turn: &str,
    resume: Option<&str>,
    binding: &mut Value,
) -> Result<(), String> {
    if let Some(original) = resume {
        let saved:Option<String>=store.db.lock().unwrap().query_row("SELECT json_extract(payload,'$.binding.taskId') FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind='session/start' ORDER BY seq LIMIT 1",params![project,original],|r|r.get(0)).optional().map_err(|e|e.to_string())?.flatten();
        if let Some(id) = saved {
            binding["taskId"] = json!(id)
        }
        // Explicit legacy retry preserves its old binding; new tasks never inherit legacy sessions.
        return Ok(());
    }
    if let Some(scope) = task_context::saved(store, project, turn)? {
        binding["taskId"] = json!(scope.task_id);
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
        let mut stmt=db.prepare("SELECT e.turn_id,json_extract(e.payload,'$.binding') FROM agent_events e WHERE e.project_id=?1 AND e.kind='session/start' AND json_extract(e.payload,'$.binding.agentId') IS ?2 AND json_extract(e.payload,'$.binding.taskId') IS ?3 AND e.seq>COALESCE((SELECT MAX(seq) FROM agent_events WHERE project_id=?1 AND kind='session/reset'),0) AND EXISTS(SELECT 1 FROM agent_messages m WHERE m.project_id=e.project_id AND m.role='assistant' AND json_extract(m.attribution,'$.turnId')=e.turn_id) ORDER BY e.seq DESC").map_err(|e|e.to_string())?;
        stmt.query_map(
            params![
                project,
                binding["agentId"].as_str(),
                binding["taskId"].as_str()
            ],
            |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)),
        )
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };
    for (turn, raw) in candidates {
        let saved: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
        if super::binding::compatible(&saved, binding) {
            return super::session::restore(store, project, &turn, binding);
        }
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
