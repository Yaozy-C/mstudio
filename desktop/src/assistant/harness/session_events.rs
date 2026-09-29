use crate::database::{Store, blobs};
use serde_json::Value;

pub fn read(
    store: &Store,
    project: &str,
    turn: &str,
    through: i64,
) -> Result<Vec<(String, Value)>, String> {
    let read = || -> anyhow::Result<Vec<(String, Value)>> {
        let db = store.db.lock().unwrap();
        let reset: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM agent_events WHERE project_id=?1 AND kind='session/reset' AND seq > COALESCE((SELECT MIN(seq) FROM agent_events WHERE project_id=?1 AND turn_id=?2),9223372036854775807))", rusqlite::params![project,turn], |r|r.get(0))?;
        anyhow::ensure!(!reset, "原会话已重置，请重新发送消息");
        let mut stmt = db.prepare("SELECT seq,kind,payload FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND seq<=?3 AND kind IN ('session/start','session/message','session/compaction','image/offload','tool/result') ORDER BY seq")?;
        let mut rows = stmt.query(rusqlite::params![project, turn, through])?;
        let mut events = Vec::new();
        let mut watermark = 0;
        while let Some(row) = rows.next()? {
            let seq: i64 = row.get(0)?;
            let kind: String = row.get(1)?;
            if kind != "session/start" && seq <= watermark {
                continue;
            }
            let value = blobs::event(&db, seq, &row.get::<_, String>(2)?)?;
            if kind == "session/start" {
                watermark = value["checkpointSeq"].as_i64().unwrap_or(seq);
                anyhow::ensure!(
                    watermark <= through,
                    "Requested history precedes recovery snapshot"
                );
            }
            events.push((kind, value));
        }
        Ok(events)
    };
    read().map_err(|e| e.to_string())
}
