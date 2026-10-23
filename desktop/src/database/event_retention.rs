//! Progress is broadcast live, while recovery, outcomes, errors and usage stay durable.
use anyhow::Result;
use rusqlite::{Connection, params};

pub const TRANSIENT: &[&str] = &[
    "assistant/partial",
    "assistant/thinking",
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
/// Caller supplies the transaction containing the terminal event.
pub fn finish(db: &Connection, project: &str, turn: &str) -> Result<()> {
    let sql = "DELETE FROM agent_events WHERE kind IN (SELECT value FROM json_each(?1))
        AND project_id=?2 AND turn_id=?3
        AND NOT EXISTS(SELECT 1 FROM agent_messages m WHERE m.project_id=agent_events.project_id
            AND json_extract(m.attribution,'$.turnId')=agent_events.turn_id
            AND m.role='assistant' AND json_extract(m.attribution,'$.status') IN ('running','waiting'))
        AND NOT EXISTS(SELECT 1 FROM subagent_runs r WHERE r.project_id=agent_events.project_id
            AND r.last_turn=agent_events.turn_id AND r.status='running')";
    let kinds = serde_json::to_string(TRANSIENT)?;
    let removed = db.execute(sql, params![kinds, project, turn])?;
    if removed > 0 {
        super::blobs::collect(db)?;
    }
    super::history_cleanup::finish(db, project, turn)?;
    Ok(())
}
