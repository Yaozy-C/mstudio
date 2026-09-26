use crate::database::Store;
use anyhow::Result;
use serde_json::{Value, json};
pub fn init(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS agent_events(seq INTEGER PRIMARY KEY AUTOINCREMENT,project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,turn_id TEXT NOT NULL,kind TEXT NOT NULL,payload TEXT NOT NULL,created INTEGER NOT NULL DEFAULT(unixepoch())); CREATE INDEX IF NOT EXISTS agent_events_project ON agent_events(project_id,seq);")?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS subagent_runs(id TEXT PRIMARY KEY,project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,parent_turn TEXT NOT NULL,parent_agent_id TEXT NOT NULL,agent_id TEXT NOT NULL,mode TEXT NOT NULL,status TEXT NOT NULL,profile TEXT NOT NULL,model_id TEXT NOT NULL,last_turn TEXT,output TEXT NOT NULL DEFAULT '',notified INTEGER NOT NULL DEFAULT 0,created INTEGER NOT NULL DEFAULT(unixepoch()),updated INTEGER NOT NULL DEFAULT(unixepoch())); CREATE INDEX IF NOT EXISTS subagent_parent ON subagent_runs(project_id,parent_turn,created); CREATE TABLE IF NOT EXISTS subagent_inbox(seq INTEGER PRIMARY KEY AUTOINCREMENT,child_id TEXT NOT NULL REFERENCES subagent_runs(id) ON DELETE CASCADE,text TEXT NOT NULL,consumed INTEGER NOT NULL DEFAULT 0,created INTEGER NOT NULL DEFAULT(unixepoch())); CREATE TABLE IF NOT EXISTS subagent_notices(seq INTEGER PRIMARY KEY AUTOINCREMENT,child_id TEXT NOT NULL REFERENCES subagent_runs(id) ON DELETE CASCADE,project_id TEXT NOT NULL,parent_agent_id TEXT NOT NULL,status TEXT NOT NULL,output TEXT NOT NULL,delivered INTEGER NOT NULL DEFAULT 0,created INTEGER NOT NULL DEFAULT(unixepoch())); CREATE INDEX IF NOT EXISTS subagent_notice_parent ON subagent_notices(project_id,parent_agent_id,delivered,seq);")?;
    let has_source: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('subagent_inbox') WHERE name='source')",
        [],
        |row| row.get(0),
    )?;
    if !has_source {
        db.execute(
            "ALTER TABLE subagent_inbox ADD COLUMN source TEXT NOT NULL DEFAULT '{}'",
            [],
        )?;
    }
    db.execute(
        "UPDATE subagent_runs SET status='ready' WHERE status='running'",
        [],
    )?;
    Ok(())
}
pub fn append(store: &Store, project: &str, turn: &str, kind: &str, payload: Value) -> Result<()> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let event_payload = if kind == "assistant/partial" {
        json!({"delta":payload["delta"]})
    } else {
        payload.clone()
    };
    tx.execute(
        "INSERT INTO agent_events(project_id,turn_id,kind,payload) VALUES(?1,?2,?3,?4)",
        rusqlite::params![project, turn, kind, event_payload.to_string()],
    )?;
    if kind == "assistant/partial" {
        let text = payload["text"].as_str().unwrap_or("");
        tx.execute("UPDATE agent_messages SET content=?3,payload=?4 WHERE project_id=?1 AND role='assistant' AND json_extract(attribution,'$.turnId')=?2 AND json_extract(attribution,'$.status')='running'", rusqlite::params![project,turn,text,json!(text).to_string()])?;
    }
    tx.commit()?;
    Ok(())
}
pub fn page(store: &Store, project: &str, before: Option<i64>) -> Result<Vec<Value>> {
    let db = store.db.lock().unwrap();
    let mut stmt=db.prepare("SELECT seq,turn_id,kind,CASE WHEN kind='tool/result' THEN json_remove(payload,'$.message','$.value') ELSE payload END,created FROM agent_events WHERE project_id=?1 AND kind NOT IN ('session/start','session/message','session/compaction','image/offload','subagent/context','subagent/configuration') AND kind != 'assistant/partial' AND seq<?2 ORDER BY seq DESC LIMIT 40")?;
    let values = stmt
        .query_map(
            rusqlite::params![project, before.unwrap_or(i64::MAX)],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, String>(3)?,
                    r.get::<_, i64>(4)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    values.into_iter().map(|(seq,turn,kind,payload,created)|Ok(serde_json::json!({"seq":seq,"turnId":turn,"kind":kind,"payload":serde_json::from_str::<Value>(&payload)?,"created":created}))).collect()
}
#[tauri::command]
pub fn agent_events(
    store: tauri::State<Store>,
    project_id: String,
    before: Option<i64>,
) -> Result<Vec<Value>, String> {
    page(&store, &project_id, before).map_err(|e| e.to_string())
}

/// A bounded factual handoff for the last run, not a replay of unverified effects.
pub fn recovery(store: &Store, project: &str) -> Result<Value> {
    use rusqlite::OptionalExtension;
    let turn: Option<String> = store.db.lock().unwrap().query_row(
        "SELECT turn_id FROM agent_events e WHERE e.project_id=?1 AND e.seq > COALESCE((SELECT max(seq) FROM agent_events WHERE project_id=?1 AND kind='session/reset'),0) AND e.kind IN ('user/message','tool/call','tool/result','turn/end') AND e.turn_id NOT LIKE 'subagent-%' AND NOT EXISTS(SELECT 1 FROM agent_messages m WHERE m.project_id=e.project_id AND m.role='assistant' AND json_extract(m.attribution,'$.turnId')=e.turn_id AND json_extract(m.attribution,'$.status')='running') ORDER BY e.seq DESC LIMIT 1",
        [project], |r| r.get(0),
    ).optional()?;
    let Some(current_turn) = turn else {
        return Ok(json!({"turnId":null,"events":[],"note":"暂无上轮执行记录"}));
    };
    let events = turn_page(store, project, &current_turn, None)?;
    let mut selected = Vec::new();
    let mut turn = None;
    let mut remaining = 5000;
    for event in events {
        if event["kind"] == "session/reset" {
            break;
        }
        let id = current_turn.clone();
        if turn.as_ref().is_some_and(|t| t != &id) {
            break;
        }
        turn = Some(id);
        if !["user/message", "tool/call", "tool/result", "turn/end"]
            .contains(&event["kind"].as_str().unwrap_or(""))
        {
            continue;
        }
        let cost = event.to_string().len();
        if cost > remaining {
            continue;
        }
        remaining -= cost;
        selected.push(event);
    }
    selected.reverse();
    Ok(
        serde_json::json!({"turnId":turn,"events":selected,"note":"仅为最近一轮的部分执行记录；缺少完成记录可能已中断，修改前 inspect 核实，不自动重放"}),
    )
}

pub fn turn_page(
    store: &Store,
    project: &str,
    turn: &str,
    before: Option<i64>,
) -> Result<Vec<Value>> {
    let db = store.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT seq,kind,CASE WHEN kind='tool/result' THEN json_remove(payload,'$.message','$.value') ELSE payload END,created FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind NOT IN ('session/start','session/message','session/compaction','image/offload','subagent/context','subagent/configuration') AND kind != 'assistant/partial' AND seq<?3 ORDER BY seq DESC LIMIT 40")?;
    let rows = stmt
        .query_map(
            rusqlite::params![project, turn, before.unwrap_or(i64::MAX)],
            |r| {
                Ok((
                    r.get::<_, i64>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                    r.get::<_, i64>(3)?,
                ))
            },
        )?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter().map(|(seq,kind,payload,created)|Ok(serde_json::json!({"seq":seq,"kind":kind,"payload":serde_json::from_str::<Value>(&payload)?,"created":created}))).collect()
}
#[tauri::command]
pub fn agent_turn_events(
    store: tauri::State<Store>,
    project_id: String,
    turn_id: String,
    before: Option<i64>,
) -> Result<Vec<Value>, String> {
    turn_page(&store, &project_id, &turn_id, before).map_err(|e| e.to_string())
}
