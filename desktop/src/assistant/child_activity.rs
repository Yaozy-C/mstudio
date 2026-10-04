//! A bounded UI projection of each child execution. It is never model history.
use crate::database::Store;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use tauri::{Emitter, Manager};

pub fn init(db: &Connection) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    let db = &tx;
    let existing: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='agent_child_activity')", [], |r| r.get(0))?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS agent_child_activity(
        project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
        turn_id TEXT NOT NULL, child_id TEXT NOT NULL REFERENCES subagent_runs(id) ON DELETE CASCADE,
        parent_turn TEXT NOT NULL, call_id TEXT NOT NULL, agent_id TEXT NOT NULL,
        state TEXT NOT NULL DEFAULT 'running', phase TEXT NOT NULL DEFAULT '{}', note TEXT NOT NULL DEFAULT '',
        started INTEGER NOT NULL DEFAULT(unixepoch()), updated INTEGER NOT NULL DEFAULT(unixepoch()),
        PRIMARY KEY(project_id,turn_id));
        CREATE INDEX IF NOT EXISTS child_activity_parent ON agent_child_activity(project_id,parent_turn);
        UPDATE agent_child_activity SET state='interrupted',updated=unixepoch() WHERE state='running';")?;
    if !existing {
        // One-time projection of settled one-shot runs. The result's childId is
        // the authoritative link; never infer a child from its role or time.
        db.execute_batch("INSERT OR IGNORE INTO agent_child_activity(project_id,turn_id,child_id,parent_turn,call_id,agent_id,state,phase,started,updated)
            SELECT r.project_id,r.last_turn,r.id,r.parent_turn,json_extract(e.payload,'$.callId'),r.agent_id,
                CASE WHEN r.status='completed' THEN 'completed' WHEN json_extract(e.payload,'$.result.stopReason')='aborted' THEN 'cancelled' ELSE 'failed' END,
                '{\"kind\":\"subagent/settled\",\"payload\":{}}',r.created,r.updated
            FROM subagent_runs r JOIN agent_events e ON e.project_id=r.project_id AND e.turn_id=r.parent_turn
                AND e.kind='tool/result' AND json_extract(e.payload,'$.result.childId')=r.id
            WHERE r.mode='oneShot' AND r.status IN ('completed','failed') AND r.last_turn IS NOT NULL AND json_type(e.payload,'$.callId')='text';")?;
    }
    tx.commit()?;
    Ok(())
}
pub fn start(
    db: &Connection,
    project: &str,
    turn: &str,
    child: &str,
    parent: &str,
    call: &str,
    agent: &str,
) -> Result<()> {
    db.execute("INSERT INTO agent_child_activity(project_id,turn_id,child_id,parent_turn,call_id,agent_id) VALUES(?1,?2,?3,?4,?5,?6)",params![project,turn,child,parent,call,agent])?;
    Ok(())
}
pub fn update(
    db: &Connection,
    project: &str,
    turn: &str,
    kind: &str,
    value: &Value,
) -> Result<Option<Value>> {
    let phase = match kind {
        "step/start" | "request/start" | "request/retry" => json!({"kind":kind,"payload":{}}),
        "tool/call" => json!({"kind":kind,"payload":{"name":value["name"],"arguments":{
            "section":value["arguments"]["section"],"skill":value["arguments"]["skill"],"path":value["arguments"]["path"],
            "operations":value["arguments"]["operations"].as_array().map(|ops| ops.iter().map(|o| json!({"op":o["op"]})).collect::<Vec<_>>())}}}),
        "tool/result" => json!({"kind":kind,"payload":{}}),
        "assistant/partial" => {
            let note = value["delta"].as_str().unwrap_or("").trim();
            if note.is_empty() {
                return Ok(None);
            }
            db.execute("UPDATE agent_child_activity SET note=?3,updated=unixepoch() WHERE project_id=?1 AND turn_id=?2 AND state='running'",params![project,turn,note.chars().take(1200).collect::<String>()])?;
            json!({"kind":kind,"payload":{}})
        }
        "subagent/settled" => {
            let state = match value["stopReason"].as_str() {
                Some("completed") => "completed",
                Some("aborted") => "cancelled",
                _ => "failed",
            };
            db.execute("UPDATE agent_child_activity SET state=?3,updated=unixepoch() WHERE project_id=?1 AND turn_id=?2",params![project,turn,state])?;
            json!({"kind":kind,"payload":{}})
        }
        _ => return Ok(None),
    };
    db.execute("UPDATE agent_child_activity SET phase=?3,updated=unixepoch() WHERE project_id=?1 AND turn_id=?2",params![project,turn,phase.to_string()])?;
    snapshot(db, project, turn)
}
fn snapshot(db: &Connection, project: &str, turn: &str) -> Result<Option<Value>> {
    Ok(db.query_row("SELECT child_id,parent_turn,call_id,agent_id,state,phase,note,started,updated FROM agent_child_activity WHERE project_id=?1 AND turn_id=?2",params![project,turn],|r| Ok(json!({
        "projectId":project,"turnId":turn,"childId":r.get::<_,String>(0)?,"parentTurn":r.get::<_,String>(1)?,"callId":r.get::<_,String>(2)?,"agentId":r.get::<_,String>(3)?,"state":r.get::<_,String>(4)?,"phase":serde_json::from_str::<Value>(&r.get::<_,String>(5)?).unwrap_or(Value::Null),"note":r.get::<_,String>(6)?,"started":r.get::<_,i64>(7)?,"updated":r.get::<_,i64>(8)?
    }))).optional()?)
}
pub fn record(tool: &super::tools::ProjectTool, kind: &str, value: &Value) -> Result<()> {
    let state = update(
        &tool.app.state::<Store>().db.lock().unwrap(),
        &tool.project,
        &tool.turn,
        kind,
        value,
    )?;
    if let Some(state) = state {
        let _ = tool.app.emit("agent-child-progress", state);
    }
    Ok(())
}
pub fn list(store: &Store, project: &str, parent: &str) -> Result<Vec<Value>> {
    let mut children = {
        let db = store.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT turn_id FROM agent_child_activity WHERE project_id=?1 AND parent_turn=?2 ORDER BY started,turn_id")?;
        let turns = stmt
            .query_map(params![project, parent], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        turns
            .iter()
            .map(|t| snapshot(&db, project, t))
            .collect::<Result<Vec<_>>>()?
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
    };
    for child in &mut children {
        child["events"] = json!(super::journal::turn_page(
            store,
            project,
            child["turnId"].as_str().unwrap(),
            None
        )?);
    }
    Ok(children)
}
#[tauri::command]
pub fn agent_child_activity(
    store: tauri::State<Store>,
    project_id: String,
    turn_id: String,
) -> Result<Vec<Value>, String> {
    list(&store, &project_id, &turn_id).map_err(|e| e.to_string())
}
#[cfg(test)]
#[path = "child_activity_tests.rs"]
mod tests;
