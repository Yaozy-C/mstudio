//! One cumulative usage row per turn; compaction is counted separately and in the total.
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

pub fn record(
    db: &Connection,
    project: &str,
    turn: &str,
    kind: &str,
    value: &Value,
) -> Result<bool> {
    if !matches!(kind, "request/usage" | "compaction/end") {
        return Ok(false);
    }
    let old: Option<(i64,String)> = db.query_row("SELECT seq,payload FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind='turn/usage'",params![project,turn],|r|Ok((r.get(0)?,r.get(1)?))).optional()?;
    let mut total: Value = old
        .as_ref()
        .map(|(_, v)| serde_json::from_str(v))
        .transpose()?
        .unwrap_or_else(|| json!({}));
    let category = if kind == "request/usage" {
        "requests"
    } else {
        "compactions"
    };
    total[category] = json!(total[category].as_u64().unwrap_or(0) + 1);
    for key in [
        "inputTokens",
        "outputTokens",
        "totalTokens",
        "cachedInputTokens",
    ] {
        total[key] = json!(
            total[key]
                .as_u64()
                .unwrap_or(0)
                .saturating_add(value[key].as_u64().unwrap_or(0))
        );
    }
    if let Some((seq, _)) = old {
        db.execute(
            "UPDATE agent_events SET payload=?1 WHERE seq=?2",
            params![total.to_string(), seq],
        )?;
    } else {
        db.execute("INSERT INTO agent_events(project_id,turn_id,kind,payload) VALUES(?1,?2,'turn/usage',?3)",params![project,turn,total.to_string()])?;
    }
    Ok(true)
}

pub fn migrate(db: &Connection) -> Result<()> {
    let rows = {
        let mut stmt = db.prepare("SELECT seq,project_id,turn_id,kind,payload FROM agent_events WHERE kind IN ('request/usage','compaction/end') ORDER BY seq")?;
        stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (seq, project, turn, kind, raw) in rows {
        record(db, &project, &turn, &kind, &serde_json::from_str(&raw)?)?;
        db.execute("DELETE FROM agent_events WHERE seq=?1", [seq])?;
    }
    db.execute_batch("CREATE UNIQUE INDEX IF NOT EXISTS agent_events_usage ON agent_events(project_id,turn_id) WHERE kind='turn/usage';")?;
    Ok(())
}
