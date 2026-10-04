use crate::database::Store;
use crate::database::event_retention;
use anyhow::Result;
use serde_json::Value;
pub fn init(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS agent_events(seq INTEGER PRIMARY KEY AUTOINCREMENT,project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,turn_id TEXT NOT NULL,kind TEXT NOT NULL,payload TEXT NOT NULL,created INTEGER NOT NULL DEFAULT(unixepoch())); CREATE INDEX IF NOT EXISTS agent_events_project ON agent_events(project_id,seq);")?;
    db.execute_batch("CREATE TABLE IF NOT EXISTS subagent_runs(id TEXT PRIMARY KEY,project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,parent_turn TEXT NOT NULL,parent_agent_id TEXT NOT NULL,agent_id TEXT NOT NULL,mode TEXT NOT NULL,status TEXT NOT NULL,profile TEXT NOT NULL,model_id TEXT NOT NULL,last_turn TEXT,output TEXT NOT NULL DEFAULT '',notified INTEGER NOT NULL DEFAULT 0,created INTEGER NOT NULL DEFAULT(unixepoch()),updated INTEGER NOT NULL DEFAULT(unixepoch())); CREATE INDEX IF NOT EXISTS subagent_parent ON subagent_runs(project_id,parent_turn,created); CREATE TABLE IF NOT EXISTS subagent_inbox(seq INTEGER PRIMARY KEY AUTOINCREMENT,child_id TEXT NOT NULL REFERENCES subagent_runs(id) ON DELETE CASCADE,text TEXT NOT NULL,consumed INTEGER NOT NULL DEFAULT 0,created INTEGER NOT NULL DEFAULT(unixepoch())); CREATE TABLE IF NOT EXISTS subagent_notices(seq INTEGER PRIMARY KEY AUTOINCREMENT,child_id TEXT NOT NULL REFERENCES subagent_runs(id) ON DELETE CASCADE,project_id TEXT NOT NULL,parent_agent_id TEXT NOT NULL,status TEXT NOT NULL,output TEXT NOT NULL,delivered INTEGER NOT NULL DEFAULT 0,created INTEGER NOT NULL DEFAULT(unixepoch())); CREATE INDEX IF NOT EXISTS subagent_notice_parent ON subagent_notices(project_id,parent_agent_id,delivered,seq);")?;
    super::child_activity::init(db)?;
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
pub fn insert(
    db: &rusqlite::Connection,
    project: &str,
    turn: &str,
    kind: &str,
    mut payload: Value,
) -> Result<()> {
    if crate::database::turn_usage::record(db, project, turn, kind, &payload)? {
        return Ok(());
    }
    if event_retention::transient(kind) {
        return Ok(());
    }
    db.execute(
        "INSERT INTO agent_events(project_id,turn_id,kind,payload) VALUES(?1,?2,?3,'{}')",
        rusqlite::params![project, turn, kind],
    )?;
    let seq = db.last_insert_rowid();
    crate::database::blobs::pack(db, crate::database::blobs::Owner::Event(seq), &mut payload)?;
    db.execute(
        "UPDATE agent_events SET payload=?1 WHERE seq=?2",
        rusqlite::params![payload.to_string(), seq],
    )?;
    Ok(())
}
pub fn append(store: &Store, project: &str, turn: &str, kind: &str, payload: Value) -> Result<()> {
    if event_retention::transient(kind) && kind != "assistant/partial" {
        return Ok(());
    }
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    insert(&tx, project, turn, kind, payload.clone())?;
    if kind == "assistant/partial" {
        let text = payload["text"].as_str().unwrap_or("");
        tx.execute("UPDATE agent_messages SET content=?3,payload=?4 WHERE project_id=?1 AND role='assistant' AND json_extract(attribution,'$.turnId')=?2 AND json_extract(attribution,'$.status')='running'", rusqlite::params![project,turn,text,serde_json::json!(text).to_string()])?;
    }
    if matches!(kind, "turn/end" | "subagent/settled") {
        event_retention::finish(&tx, project, turn)?;
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
    values.into_iter().map(|(seq,turn,kind,payload,created)|Ok(serde_json::json!({"seq":seq,"turnId":turn,"kind":kind,"payload":crate::database::blobs::event(&db,seq,&payload)?,"created":created}))).collect()
}
#[tauri::command]
pub fn agent_events(
    store: tauri::State<Store>,
    project_id: String,
    before: Option<i64>,
) -> Result<Vec<Value>, String> {
    page(&store, &project_id, before).map_err(|e| e.to_string())
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
    rows.into_iter().map(|(seq,kind,payload,created)|Ok(serde_json::json!({"seq":seq,"kind":kind,"payload":crate::database::blobs::event(&db,seq,&payload)?,"created":created}))).collect()
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
