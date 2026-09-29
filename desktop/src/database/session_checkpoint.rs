//! A session/start becomes the latest recovery snapshot, independent of tool-log retention.
//! Keep its original sequence for reset/role selection and record the replay watermark.
use super::blobs::{self, Owner};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

pub fn fold(db: &Connection, project: &str, turn: &str, through: i64) -> Result<()> {
    let start: Option<(i64, String)> = db.query_row(
        "SELECT seq,payload FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind='session/start' AND seq<=?3 ORDER BY seq DESC LIMIT 1",
        params![project,turn,through], |r| Ok((r.get(0)?,r.get(1)?)),
    ).optional()?;
    let Some((seq, raw)) = start else {
        return Ok(());
    };
    let packed: Value = serde_json::from_str(&raw)?;
    let after = packed["checkpointSeq"].as_i64().unwrap_or(seq);
    if after >= through {
        return Ok(());
    }
    let mut snapshot = blobs::hydrate(db, Owner::Event(seq), packed)?;
    let messages = snapshot["messages"]
        .as_array_mut()
        .context("Invalid session snapshot")?;
    let rows = {
        let mut stmt = db.prepare("SELECT seq,kind,payload FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND seq>?3 AND seq<=?4 AND kind IN ('session/message','session/compaction','image/offload','tool/result') ORDER BY seq")?;
        stmt.query_map(params![project, turn, after, through], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, kind, raw) in rows {
        let value = blobs::event(db, id, &raw)?;
        match kind.as_str() {
            "session/compaction" => {
                let start = value["start"]
                    .as_u64()
                    .context("Invalid compaction start")? as usize;
                let end = value["end"].as_u64().context("Invalid compaction end")? as usize;
                ensure!(
                    start < end && end <= messages.len(),
                    "Invalid compaction range"
                );
                messages.splice(start..end, [value["message"].clone()]);
            }
            "image/offload" => {
                let index = value["messageIndex"]
                    .as_u64()
                    .context("Invalid image position")? as usize;
                *messages.get_mut(index).context("Invalid image rewrite")? =
                    value["message"].clone();
            }
            _ => {
                if let Some(message) = value.get("message") {
                    messages.push(message.clone());
                }
            }
        }
    }
    snapshot["checkpointSeq"] = json!(through);
    blobs::pack(db, Owner::Event(seq), &mut snapshot)?;
    db.execute(
        "UPDATE agent_events SET payload=?1 WHERE seq=?2",
        params![snapshot.to_string(), seq],
    )?;
    db.execute("DELETE FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND seq<=?3 AND (kind IN ('session/message','session/compaction') OR (kind='session/start' AND seq!=?4))", params![project,turn,through,seq])?;
    // Result views remain available for seven days; provider messages are now in the snapshot.
    db.execute("DELETE FROM event_content WHERE owner IN (SELECT seq FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND seq<=?3 AND kind IN ('tool/result','image/offload')) AND (pointer='/message' OR pointer LIKE '/message/%')",params![project,turn,through])?;
    db.execute("UPDATE agent_events SET payload=json_remove(payload,'$.message') WHERE project_id=?1 AND turn_id=?2 AND seq<=?3 AND kind IN ('tool/result','image/offload')",params![project,turn,through])?;
    Ok(())
}

/// Completed predecessors are superseded, but interrupted tasks and child continuations survive.
pub fn prune_superseded(db: &Connection) -> Result<()> {
    // Continuations are scoped by child instance, never by role alone.
    db.execute("DELETE FROM agent_events AS s WHERE s.kind='session/start' AND EXISTS(SELECT 1 FROM agent_events e JOIN subagent_runs r ON r.project_id=e.project_id AND r.id=json_extract(e.payload,'$.childId') WHERE e.project_id=s.project_id AND e.turn_id=s.turn_id AND e.kind='subagent/settled' AND r.last_turn!=s.turn_id AND EXISTS(SELECT 1 FROM agent_events n WHERE n.project_id=r.project_id AND n.turn_id=r.last_turn AND n.kind='session/start' AND json_type(n.payload,'$.checkpointSeq')='integer'))", [])?;
    let rows = {
        let mut stmt=db.prepare("SELECT seq,project_id,turn_id,json_extract(payload,'$.binding') FROM agent_events WHERE kind='session/start' AND json_type(payload,'$.checkpointSeq')='integer' ORDER BY seq DESC")?;
        stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    let mut seen = std::collections::HashSet::new();
    for (seq, project, turn, raw) in rows {
        let mut binding: Value = serde_json::from_str(&raw)?;
        if let Some(object) = binding.as_object_mut() {
            object.remove("revision");
            object.remove("tools");
        }
        // Old unscoped conversations cannot safely be grouped as one task.
        if binding["taskId"].is_null() {
            continue;
        }
        let completed: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM agent_messages WHERE project_id=?1 AND role='assistant' AND json_extract(attribution,'$.turnId')=?2 AND json_extract(attribution,'$.status')='completed') AND NOT EXISTS(SELECT 1 FROM subagent_runs WHERE project_id=?1 AND last_turn=?2)",params![project,turn],|r|r.get(0))?;
        if completed && !seen.insert((project, binding.to_string())) {
            db.execute("DELETE FROM agent_events WHERE seq=?1", [seq])?;
        }
    }
    Ok(())
}

/// Authorization survives removal of an obsolete recovery snapshot.
pub fn owner(db: &Connection, project: &str, turn: &str) -> Result<Option<String>> {
    Ok(db.query_row("SELECT COALESCE((SELECT json_extract(payload,'$.binding.agentId') FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind='session/start' ORDER BY seq DESC LIMIT 1),(SELECT json_extract(attribution,'$.agentId') FROM agent_messages WHERE project_id=?1 AND json_extract(attribution,'$.turnId')=?2 AND role='assistant' LIMIT 1),(SELECT r.agent_id FROM agent_events e JOIN subagent_runs r ON r.id=json_extract(e.payload,'$.childId') AND r.project_id=e.project_id WHERE e.project_id=?1 AND e.turn_id=?2 AND e.kind='subagent/settled' ORDER BY e.seq DESC LIMIT 1))",params![project,turn],|r|r.get(0))?)
}
