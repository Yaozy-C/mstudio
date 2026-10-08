//! Generated media that exists but whose pixels have not been inspected yet.
//!
//! A completed generation proves the model finished; it does not prove the clip
//! is usable. This module records each imported result together with the intent
//! that produced it, settles the record when actual frames are read, and offers a
//! compact list for the Agent's project snapshot so a delivered result can be
//! judged instead of silently accepted.
use crate::database::Store;
use anyhow::Result;
use rusqlite::params;
use serde_json::{Value, json};

/// Results older than this are no longer surfaced; stale entries stay harmless.
const FRESH_SECONDS: i64 = 7 * 24 * 60 * 60;
const MAX_LISTED: usize = 8;

pub fn init(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS unverified_results(
            project_id TEXT NOT NULL,
            asset_id TEXT NOT NULL,
            task_key TEXT NOT NULL,
            kind TEXT NOT NULL,
            prompt TEXT NOT NULL,
            references_json TEXT NOT NULL,
            created INTEGER NOT NULL DEFAULT(unixepoch()),
            settled INTEGER,
            settled_reason TEXT,
            PRIMARY KEY(project_id, asset_id));",
    )?;
    Ok(())
}

/// Record an imported result against the task that requested it.
pub fn record(store: &Store, project: &str, job: &Value, asset: &Value) -> Result<()> {
    let Some(asset_id) = asset["id"].as_str().filter(|id| !id.is_empty()) else {
        return Ok(());
    };
    let task = &job["shot"]["canvasGeneration"]["task"];
    let kind = task["kind"]
        .as_str()
        .or_else(|| asset["kind"].as_str())
        .unwrap_or_default();
    let references = task["inputs"]
        .as_array()
        .map(|inputs| {
            json!(
                inputs
                    .iter()
                    .filter(|i| i["role"] != "script")
                    .map(|i| json!({
                        "assetId": i["assetId"],
                        "role": i["role"],
                        "purpose": i["purpose"],
                    }))
                    .collect::<Vec<_>>()
            )
        })
        .unwrap_or_else(|| json!([]));
    store.db.lock().unwrap().execute(
        "INSERT INTO unverified_results(project_id,asset_id,task_key,kind,prompt,references_json)
         VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(project_id,asset_id) DO NOTHING",
        params![
            project,
            asset_id,
            task["key"].as_str().unwrap_or_default(),
            kind,
            task["prompt"].as_str().unwrap_or_default(),
            references.to_string(),
        ],
    )?;
    Ok(())
}

/// Reading actual pixels is the evidence the record was waiting for.
pub fn settle(store: &Store, project: &str, asset_id: &str, reason: &str) {
    let _ = store.db.lock().unwrap().execute(
        "UPDATE unverified_results SET settled=unixepoch(),settled_reason=?3
         WHERE project_id=?1 AND asset_id=?2 AND settled IS NULL",
        params![project, asset_id, reason],
    );
}

/// Compact, bounded summary for the Agent's snapshot; empty when nothing is due.
pub fn pending(store: &Store, project: &str) -> Value {
    let db = store.db.lock().unwrap();
    let Ok(mut stmt) = db.prepare(
        "SELECT asset_id,task_key,kind,substr(prompt,1,400),references_json
         FROM unverified_results
         WHERE project_id=?1 AND settled IS NULL AND created>unixepoch()-?2
         ORDER BY created DESC LIMIT ?3",
    ) else {
        return json!([]);
    };
    let items = stmt
        .query_map(params![project, FRESH_SECONDS, MAX_LISTED as i64], |r| {
            Ok(json!({
                "assetId": r.get::<_, String>(0)?,
                "taskKey": r.get::<_, String>(1)?,
                "kind": r.get::<_, String>(2)?,
                "requestedPrompt": r.get::<_, String>(3)?,
                "references": serde_json::from_str::<Value>(&r.get::<_, String>(4)?)
                    .unwrap_or(Value::Null),
            }))
        })
        .map(|rows| rows.filter_map(std::result::Result::ok).collect::<Vec<_>>())
        .unwrap_or_default();
    json!(items)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn store() -> (std::path::PathBuf, Store) {
        let root =
            std::env::temp_dir().join(format!("mstudio-result-check-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        (root, store)
    }

    fn job() -> Value {
        json!({"shot":{"canvasGeneration":{"task":{
        "key":"run-1","kind":"video","prompt":"Lower the bag onto the desk.",
        "inputs":[
            {"assetId":"a","role":"reference","purpose":"the lead's face identity"},
            {"assetId":"b","role":"script","purpose":"brief"}
        ]}}}})
    }

    #[test]
    fn a_produced_result_stays_listed_until_its_frames_are_read() {
        let (root, store) = store();
        record(&store, "p", &job(), &json!({"id":"out-1","kind":"video"})).unwrap();
        // Recording is idempotent and keeps the original intent.
        record(&store, "p", &job(), &json!({"id":"out-1","kind":"video"})).unwrap();
        let listed = pending(&store, "p");
        assert_eq!(listed.as_array().unwrap().len(), 1);
        assert_eq!(listed[0]["assetId"], "out-1");
        assert_eq!(listed[0]["taskKey"], "run-1");
        assert_eq!(listed[0]["references"].as_array().unwrap().len(), 1);
        assert_eq!(
            listed[0]["references"][0]["purpose"],
            "the lead's face identity"
        );
        settle(&store, "p", "out-1", "frames_read");
        assert!(pending(&store, "p").as_array().unwrap().is_empty());
        // Another project is unaffected.
        record(&store, "q", &job(), &json!({"id":"out-2","kind":"image"})).unwrap();
        assert_eq!(pending(&store, "p").as_array().unwrap().len(), 0);
        assert_eq!(pending(&store, "q").as_array().unwrap().len(), 1);
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
