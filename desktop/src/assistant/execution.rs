//! Parent response lifecycle is independent of one IPC wait or model run.
use crate::database::Store;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Serialize;
use serde_json::json;
use tauri::{Emitter, Manager};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct State {
    pub project_id: String,
    pub turn_id: String,
    pub status: String,
    pub text: String,
    pub error: Option<String>,
    pub revision: i64,
}

#[derive(Serialize)]
pub struct Reply {
    pub text: String,
    pub status: String,
}

pub fn pending(db: &Connection, project: &str, turn: &str) -> Result<bool> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM subagent_runs r WHERE r.project_id=?1 AND r.parent_turn=?2 AND (r.status='running' OR EXISTS(SELECT 1 FROM subagent_notices n WHERE n.child_id=r.id AND n.delivered=0)))",
        params![project, turn], |r| r.get(0),
    )?)
}

pub fn read(store: &Store, project: &str) -> Result<Option<State>> {
    Ok(store.db.lock().unwrap().query_row(
        "SELECT json_extract(attribution,'$.turnId'),json_extract(attribution,'$.status'),content,json_extract(attribution,'$.error'),coalesce((SELECT max(e.seq) FROM agent_events e WHERE e.project_id=m.project_id AND e.turn_id=json_extract(m.attribution,'$.turnId')),0) FROM agent_messages m WHERE project_id=?1 AND role='assistant' AND json_type(attribution,'$.turnId')='text' ORDER BY id DESC LIMIT 1",
        [project], |r| Ok(State { project_id: project.into(), turn_id: r.get(0)?, status: r.get(1)?, text: r.get(2)?, error: r.get(3)?, revision:r.get(4)? }),
    ).optional()?)
}

pub fn publish(app: &tauri::AppHandle, project: &str) -> Result<()> {
    let store = app.state::<Store>();
    if let Some(state) = read(&store, project)? {
        super::journal::append(
            &store,
            project,
            &state.turn_id,
            "agent/status",
            json!({"status":state.status}),
        )?;
        if let Some(state) = read(&store, project)? {
            let _ = app.emit("agent-execution", state);
        }
    }
    Ok(())
}

/// A parked parent has no PendingTurn token. Cancel its durable wait as well.
pub fn cancel_waiting(app: &tauri::AppHandle, project: &str) -> Result<()> {
    let store = app.state::<Store>();
    for state in end_saved(
        &store,
        project,
        None,
        "cancelled",
        &crate::app_error::cancelled(),
    )? {
        emit(app, state);
    }
    Ok(())
}

pub fn fail_delivery(app: &tauri::AppHandle, project: &str, turn: &str, error: &str) -> Result<()> {
    for state in end_saved(&app.state::<Store>(), project, Some(turn), "failed", error)? {
        emit(app, state);
    }
    Ok(())
}

/// Persist the terminal edge together with the response. A canceled response
/// must never be resurrected by a late background failure or completion.
fn end_saved(
    store: &Store,
    project: &str,
    turn: Option<&str>,
    status: &str,
    error: &str,
) -> Result<Vec<State>> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let rows = {
        let mut stmt = tx.prepare("SELECT id,json_extract(attribution,'$.turnId'),content FROM agent_messages WHERE project_id=?1 AND role='assistant' AND (?2 IS NULL OR json_extract(attribution,'$.turnId')=?2) AND (json_extract(attribution,'$.status')='waiting' OR (?2 IS NOT NULL AND json_extract(attribution,'$.status')='running'))")?;
        stmt.query_map(params![project, turn], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut states = vec![];
    for (id, turn, text) in rows {
        tx.execute("UPDATE agent_messages SET attribution=json_set(attribution,'$.status',?2,'$.error',?3) WHERE id=?1", params![id,status,error])?;
        super::journal::insert(
            &tx,
            project,
            &turn,
            "turn/end",
            json!({"status":status,"error":error}),
        )?;
        let revision = tx.query_row(
            "SELECT max(seq) FROM agent_events WHERE project_id=?1 AND turn_id=?2",
            params![project, turn],
            |r| r.get(0),
        )?;
        crate::database::event_retention::finish(&tx, project, &turn)?;
        states.push(State {
            project_id: project.into(),
            turn_id: turn,
            status: status.into(),
            text,
            error: Some(error.into()),
            revision,
        });
    }
    tx.commit()?;
    Ok(states)
}

fn emit(app: &tauri::AppHandle, state: State) {
    let kind = if state.status == "waiting" {
        "turn/waiting"
    } else {
        "turn/end"
    };
    let _ = app.emit("agent-progress", json!({"projectId":state.project_id,"turnId":state.turn_id,"kind":kind,"payload":{"status":state.status,"error":state.error}}));
    let _ = app.emit("agent-execution", state);
}

pub fn finish(
    app: &tauri::AppHandle,
    project: &str,
    turn: &str,
    result: &Result<String, String>,
    token: &tokio_util::sync::CancellationToken,
) -> Result<String> {
    let store = app.state::<Store>();
    let state = finish_saved(&store, project, turn, result, token)?;
    let status = state.status.clone();
    emit(app, state);
    Ok(status)
}

fn finish_saved(
    store: &Store,
    project: &str,
    turn: &str,
    result: &Result<String, String>,
    token: &tokio_util::sync::CancellationToken,
) -> Result<State> {
    let mut db = store.conn()?;
    let tx = db.transaction()?;
    let partial = super::harness::session::partial_in(&tx, project, turn)?;
    let status = if token.is_cancelled() {
        "cancelled"
    } else if result.is_err() {
        "failed"
    } else if pending(&tx, project, turn)? {
        "waiting"
    } else {
        "completed"
    };
    let text = result.as_deref().unwrap_or(&partial);
    super::history::finish_in(
        &tx,
        project,
        turn,
        text,
        status,
        result.as_ref().err().map(String::as_str),
    )?;
    let kind = if status == "waiting" {
        "turn/waiting"
    } else {
        "turn/end"
    };
    let payload = json!({"status":status,"error":result.as_ref().err()});
    super::journal::insert(&tx, project, turn, kind, payload)?;
    let revision = tx.query_row(
        "SELECT max(seq) FROM agent_events WHERE project_id=?1 AND turn_id=?2",
        params![project, turn],
        |r| r.get(0),
    )?;
    if kind == "turn/end" {
        crate::database::event_retention::finish(&tx, project, turn)?;
    }
    tx.commit()?;
    Ok(State {
        project_id: project.into(),
        turn_id: turn.into(),
        status: status.into(),
        text: text.into(),
        error: result.as_ref().err().cloned(),
        revision,
    })
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;

#[tauri::command]
pub fn agent_execution_state(
    store: tauri::State<Store>,
    project_id: String,
) -> Result<Option<State>, String> {
    read(&store, &project_id).map_err(|e| e.to_string())
}
