//! Media resolution happens before the transaction; message and acknowledgement commit together.
use crate::assistant::{config::Profile, profiles::AgentProfile};
use crate::database::Store;
use rig_core::message::Message;
use serde_json::{Value, json};

pub fn agent_source(agent: &str, turn: &str) -> Value {
    json!({"kind":"agent-message","form":"relay","senderAgentId":agent,"senderTurnId":turn})
}

pub fn take_inbox(
    store: &Store,
    child_id: &str,
    project: &str,
    turn: &str,
    model: &Profile,
    role: &AgentProfile,
) -> Result<Vec<Message>, String> {
    let (rows, doc) = {
        let db = store.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT i.seq,i.text,i.source FROM subagent_inbox i JOIN subagent_runs r ON r.id=i.child_id WHERE i.child_id=?1 AND r.project_id=?2 AND i.consumed=0 ORDER BY i.seq").map_err(|e|e.to_string())?;
        let rows = stmt
            .query_map(rusqlite::params![child_id, project], |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| e.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| e.to_string())?;
        if rows.is_empty() {
            return Ok(vec![]);
        }
        let doc = crate::project_service::load(&db, project).map_err(|e| e.to_string())?;
        (rows, doc)
    };
    let mut forwarded = std::collections::HashSet::new();
    let messages = rows
        .into_iter()
        .map(|(seq, text, raw)| {
            let source: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
            let request_turn = source["requestTurnId"].as_str()
                .or_else(|| source["senderTurnId"].as_str()).unwrap_or_default().to_owned();
            let refs = super::handoff_artifacts::user_refs(&source["originalRequest"]);
            let already_sent: bool = store.db.lock().unwrap().query_row(
                "SELECT EXISTS(SELECT 1 FROM agent_events WHERE project_id=?1 AND kind='media/forwarded' AND json_extract(payload,'$.childId')=?2 AND json_extract(payload,'$.requestTurnId')=?3)",
                rusqlite::params![project,child_id,request_turn], |r|r.get(0)).map_err(|e|e.to_string())?;
            let send = !refs.is_empty() && !already_sent && forwarded.insert(request_turn.clone());
            let mut input = source.clone();
            if !send && !refs.is_empty() {
                input["attachmentsAlreadyProvided"] = json!(true);
                input["originalRequest"]["refs"] = json!([]);
                if let Some(all) = input["refs"].as_array_mut() {
                    all.retain(|r| !refs.iter().any(|old| r["kind"]==old.kind && r["id"]==old.id));
                }
            }
            let message =
                super::handoff_artifacts::message(store, &doc, model, role, &text, &input)?;
            Ok((seq, source, message, send.then_some(request_turn)))
        })
        .collect::<Result<Vec<_>, String>>()?;
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let mut delivered = Vec::new();
    for (seq, source, message, forwarded) in messages {
        let changed = tx
            .execute(
                "UPDATE subagent_inbox SET consumed=1 WHERE seq=?1 AND consumed=0",
                [seq],
            )
            .map_err(|e| e.to_string())?;
        if changed == 0 {
            continue;
        }
        if let Some(request_turn) = forwarded {
            crate::assistant::journal::insert(
                &tx,
                project,
                turn,
                "media/forwarded",
                json!({"childId":child_id,"requestTurnId":request_turn}),
            )
            .map_err(|e| e.to_string())?;
        }
        for (kind, key) in [
            ("subagent/requirements", "expectedGenerations"),
            ("request/source", "requestTurnId"),
        ] {
            if !source[key].is_null() {
                crate::assistant::journal::insert(
                    &tx,
                    project,
                    turn,
                    kind,
                    json!({key:source[key]}),
                )
                .map_err(|e| e.to_string())?;
            }
        }
        crate::assistant::journal::insert(
            &tx,
            project,
            turn,
            "session/message",
            json!({"message":message,"source":source}),
        )
        .map_err(|e| e.to_string())?;
        delivered.push(message);
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(delivered)
}

// Same-turn notices and earlier turns in the same task are eligible; a new task is isolated.
const NOTICES: &str = "SELECT n.seq,n.child_id,n.status,n.output FROM subagent_notices n
JOIN subagent_runs r ON r.id=n.child_id
LEFT JOIN agent_messages parent ON parent.project_id=r.project_id AND parent.role='user' AND json_extract(parent.attribution,'$.turnId')=r.parent_turn
LEFT JOIN agent_messages current ON current.project_id=n.project_id AND current.role='user' AND json_extract(current.attribution,'$.turnId')=?3
WHERE n.project_id=?1 AND n.parent_agent_id=?2 AND n.delivered=0
AND (r.parent_turn=?3 OR json_extract(parent.attribution,'$.taskScope.taskId')=json_extract(current.attribution,'$.taskScope.taskId')) ORDER BY n.seq";

pub fn pending(
    db: &rusqlite::Connection,
    project: &str,
    agent: &str,
    turn: &str,
) -> Result<bool, String> {
    db.query_row(
        &format!("SELECT EXISTS({NOTICES})"),
        rusqlite::params![project, agent, turn],
        |r| r.get(0),
    )
    .map_err(|e| e.to_string())
}
pub fn notices(
    store: &Store,
    project: &str,
    agent: &str,
    turn: &str,
) -> Result<Vec<Message>, String> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let rows = {
        let mut stmt = tx.prepare(NOTICES).map_err(|e| e.to_string())?;
        stmt.query_map(rusqlite::params![project, agent, turn], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };
    for (seq, id, status, output) in &rows {
        let message = notice_message(id, status, output);
        crate::assistant::journal::insert(&tx, project, turn, "session/message", json!({"message":message,"source":{"kind":"subagent-settled","form":"notice","senderAgentId":id,"status":status}})).map_err(|e|e.to_string())?;
        tx.execute(
            "UPDATE subagent_notices SET delivered=1 WHERE seq=?1",
            [seq],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(_, id, status, output)| notice_message(&id, &status, &output))
        .collect())
}

fn notice_message(id: &str, status: &str, output: &str) -> Message {
    Message::user(format!(
        "Subagent {id} settled this turn with status {status}. Final message: {}",
        output
    ))
}

#[cfg(test)]
#[path = "mailbox_tests.rs"]
mod tests;
