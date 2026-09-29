//! Progress is broadcast live, while recovery, outcomes, errors and usage stay durable.
use anyhow::Result;
use rusqlite::{Connection, params};

pub const TRANSIENT: &[&str] = &[
    "assistant/partial",
    "user/message",
    "assistant/message",
    "skill/read",
    "skill/loaded",
    "context/selection",
    "tool/start",
    "turn/start",
    "subagent/created",
    "request/start",
    "step/start",
    "step/end",
    "context/usage",
    "compaction/start",
    "request/context",
    "subagent/context",
    "delegate/request/start",
    "delegate/step/start",
    "delegate/step/end",
];

pub fn transient(kind: &str) -> bool {
    TRANSIENT.contains(&kind)
}

// Explicit allowlist: new/unknown event kinds remain durable by default.
fn remove(db: &Connection, scope: Option<(&str, &str)>) -> Result<usize> {
    let scope_sql = if scope.is_some() {
        "AND project_id=?2 AND turn_id=?3"
    } else {
        ""
    };
    let sql = format!(
        "DELETE FROM agent_events WHERE kind IN (SELECT value FROM json_each(?1)) {scope_sql}
        AND NOT EXISTS(SELECT 1 FROM agent_messages m WHERE m.project_id=agent_events.project_id
            AND json_extract(m.attribution,'$.turnId')=agent_events.turn_id
            AND m.role='assistant' AND json_extract(m.attribution,'$.status')='running')
        AND NOT EXISTS(SELECT 1 FROM subagent_runs r WHERE r.project_id=agent_events.project_id
            AND r.last_turn=agent_events.turn_id AND r.status='running')"
    );
    let kinds = serde_json::to_string(TRANSIENT)?;
    let removed = match scope {
        Some((project, turn)) => db.execute(&sql, params![kinds, project, turn])?,
        None => db.execute(&sql, [kinds])?,
    };
    if removed > 0 {
        super::blobs::collect(db)?;
    }
    Ok(removed)
}

/// Caller supplies the transaction containing the terminal event.
pub fn finish(db: &Connection, project: &str, turn: &str) -> Result<()> {
    remove(db, Some((project, turn)))?;
    super::history_cleanup::finish(db, project, turn)?;
    Ok(())
}

pub fn migrate(db: &Connection) -> Result<()> {
    let done: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM database_migrations WHERE version=2)",
        [],
        |r| r.get(0),
    )?;
    if done {
        return Ok(());
    }
    let tx = db.unchecked_transaction()?;
    let removed = remove(&tx, None)?;
    tx.execute("INSERT INTO database_migrations(version) VALUES(2)", [])?;
    tx.commit()?;
    if removed > 0 {
        db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM; PRAGMA optimize;")?;
    }
    Ok(())
}
