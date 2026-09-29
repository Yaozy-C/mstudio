//! Bounded execution history. Recovery snapshots and their referenced results are durable.
use super::{blobs, event_retention, session_checkpoint, turn_usage};
use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::Value;

pub const ACTIVE: &str = "NOT EXISTS(SELECT 1 FROM agent_messages m WHERE m.project_id=e.project_id AND json_extract(m.attribution,'$.turnId')=e.turn_id AND m.role='assistant' AND json_extract(m.attribution,'$.status')='running') AND NOT EXISTS(SELECT 1 FROM subagent_runs r WHERE r.project_id=e.project_id AND r.last_turn=e.turn_id AND r.status='running')";

pub fn run(db: &Connection, now: i64) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    let turns = {
        let mut stmt=tx.prepare(&format!("SELECT e.project_id,e.turn_id,CASE WHEN MAX(CASE WHEN e.kind IN ('turn/end','subagent/settled') THEN e.seq END)>=MAX(CASE WHEN e.kind='session/start' THEN e.seq END) THEN MAX(CASE WHEN e.kind IN ('turn/end','subagent/settled') THEN e.seq END) ELSE MAX(e.seq) END FROM agent_events e WHERE {ACTIVE} AND e.kind IN ('session/start','session/message','session/compaction','image/offload','tool/result','turn/end','subagent/settled') GROUP BY e.project_id,e.turn_id"))?;
        stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, i64>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (project, turn, through) in turns {
        session_checkpoint::fold(&tx, &project, &turn, through)?;
    }
    turn_usage::migrate(&tx)?;
    tx.execute(&format!("DELETE FROM agent_events AS e WHERE kind IN (SELECT value FROM json_each(?1)) AND {ACTIVE}"),[serde_json::to_string(event_retention::TRANSIENT)?])?;
    session_checkpoint::prune_superseded(&tx)?;
    expire(&tx, now)?;
    blobs::collect(&tx)?;
    tx.commit()?;
    Ok(())
}

fn reference_text(value: &Value, texts: &mut String) {
    match value {
        Value::String(text)
            if text.contains("resultRef")
                || text.contains("callId")
                || text.contains("<compacted-summary>") =>
        {
            texts.push_str(text);
            texts.push('\n');
        }
        Value::Array(values) => {
            for value in values {
                reference_text(value, texts);
            }
        }
        Value::Object(values) => {
            for value in values.values() {
                reference_text(value, texts);
            }
        }
        _ => {}
    }
}
fn expire(db: &Connection, now: i64) -> Result<()> {
    // Use the last event, not a call's age: a long-running/retried turn gets a fresh 7 days.
    let rows = {
        let mut stmt=db.prepare(&format!("SELECT e.seq,e.project_id,e.kind,e.payload FROM agent_events e WHERE {ACTIVE} AND e.kind IN ('tool/call','tool/result','delegate/tool/call','delegate/tool/result','request/retry','model/stop','compaction/warning','subagent/configuration') AND (SELECT MAX(created) FROM agent_events x WHERE x.project_id=e.project_id AND x.turn_id=e.turn_id AND x.kind!='turn/usage')<=?1"))?;
        stmt.query_map([now - 7 * 24 * 60 * 60], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    if rows.is_empty() {
        return Ok(());
    }
    let mut references = std::collections::HashMap::<String, String>::new();
    let snapshots = {
        let mut stmt=db.prepare("SELECT seq,project_id,payload FROM agent_events WHERE kind IN ('session/start','session/message','session/compaction')")?;
        stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (seq, project, raw) in snapshots {
        reference_text(
            &blobs::text_event(db, seq, &raw)?,
            references.entry(project).or_default(),
        );
    }
    // Follow references transitively: a retained result may refer to another result.
    let mut pending = rows;
    loop {
        let mut next = Vec::new();
        let mut retained = false;
        for (seq, project, kind, raw) in pending {
            let value: Value = serde_json::from_str(&raw)?;
            let pinned = kind == "tool/result"
                && value["callId"].as_str().is_some_and(|id| {
                    !id.is_empty()
                        && references
                            .get(&project)
                            .is_some_and(|text| text.contains(id))
                });
            if pinned {
                reference_text(
                    &blobs::text_event(db, seq, &raw)?,
                    references.entry(project).or_default(),
                );
                retained = true;
            } else {
                next.push((seq, project, kind, raw));
            }
        }
        if !retained {
            for (seq, _, _, _) in next {
                db.execute("DELETE FROM agent_events WHERE seq=?1", [seq])?;
            }
            break;
        }
        pending = next;
    }
    Ok(())
}

pub fn startup(db: &Connection) -> Result<()> {
    let fresh: bool = db.query_row(
        "SELECT NOT EXISTS(SELECT 1 FROM database_migrations WHERE version=4)",
        [],
        |r| r.get(0),
    )?;
    let now = db.query_row("SELECT unixepoch()", [], |r| r.get(0))?;
    run(db, now)?;
    if fresh {
        db.execute("INSERT INTO database_migrations(version) VALUES(4)", [])?;
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM; PRAGMA optimize;")?;
    }
    Ok(())
}

pub fn finish(db: &Connection, project: &str, turn: &str) -> Result<()> {
    let active:bool=db.query_row(&format!("SELECT EXISTS(SELECT 1 FROM agent_events e WHERE e.project_id=?1 AND e.turn_id=?2 AND NOT ({ACTIVE}))"),params![project,turn],|r|r.get(0))?;
    if active {
        return Ok(());
    }
    let through = db.query_row(
        "SELECT MAX(seq) FROM agent_events WHERE project_id=?1 AND turn_id=?2",
        params![project, turn],
        |r| r.get(0),
    )?;
    session_checkpoint::fold(db, project, turn, through)?;
    session_checkpoint::prune_superseded(db)?;
    blobs::collect(db)
}

/// The app may stay open for weeks. Expiration runs locally without model requests.
pub fn start(app: tauri::AppHandle) {
    use tauri::Manager;
    tauri::async_runtime::spawn(async move {
        loop {
            tokio::time::sleep(std::time::Duration::from_secs(3600)).await;
            let app = app.clone();
            let result = tauri::async_runtime::spawn_blocking(move || -> Result<()> {
                let store = app.state::<super::Store>();
                let _files = store.files.blocking_read();
                let root = store.media_root();
                let db = store.db.lock().unwrap();
                let now = db.query_row("SELECT unixepoch()", [], |r| r.get(0))?;
                run(&db, now)?;
                crate::project_storage::resume_cleanup(&db, &root)?;
                db.execute_batch("PRAGMA wal_checkpoint(PASSIVE); PRAGMA optimize;")?;
                Ok(())
            })
            .await;
            if !matches!(result, Ok(Ok(()))) {
                eprintln!("History maintenance failed: {result:?}");
            }
        }
    });
}
