//! Project task state is the durable outbox. Claiming and updating a task are
//! database transactions; a view opening/closing never dispatches a paid job.
use super::{load, persist, runtime};
use crate::database::Store;
use anyhow::{Context, Result, ensure};
use rusqlite::Connection;
use serde_json::{Value, json};
pub fn candidates(db: &Connection) -> Result<Vec<(String, String)>> {
    let mut statement = db.prepare("SELECT p.id, t.key FROM projects p, json_each(p.document, '$.production.drafts') t WHERE json_extract(t.value,'$.status')='READY' ORDER BY json_extract(t.value,'$.createdAt'),p.id,t.key LIMIT 100")?;
    Ok(statement
        .query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?)
}
pub fn claim(store: &Store, project: &str, key: &str) -> Result<Option<Value>> {
    let mut db = store.conn()?;
    let tx = db.transaction()?;
    let before = load(&tx, project)?;
    let mut after = before.clone();
    let task = &mut after["production"]["drafts"][key];
    if task["status"] != "READY" {
        return Ok(None);
    }
    if !task["submissionId"].is_string() {
        task["submissionId"] = json!(mstudio::media::id());
    }
    task["status"] = json!("UPLOADING");
    let claimed = task.clone();
    persist(&tx, &before, &after)?;
    tx.commit()?;
    store.project_changed(project);
    Ok(Some(claimed))
}
pub fn patch(
    store: &Store,
    project: &str,
    key: &str,
    submission: &str,
    expected: &[&str],
    patch: Value,
) -> Result<bool> {
    let mut db = store.conn()?;
    let tx = db.transaction()?;
    let before = load(&tx, project)?;
    let mut after = before.clone();
    let task = &mut after["production"]["drafts"][key];
    if task["submissionId"] != submission
        || !expected.contains(&task["status"].as_str().unwrap_or(""))
    {
        return Ok(false);
    }
    let fields = task.as_object_mut().context("Invalid task")?;
    for (k, v) in patch.as_object().context("Invalid task patch")? {
        if v.is_null() {
            fields.remove(k);
        } else {
            fields.insert(k.clone(), v.clone());
        }
    }
    persist(&tx, &before, &after)?;
    tx.commit()?;
    store.project_changed(project);
    Ok(true)
}
pub fn prepare(
    store: &Store,
    project: &str,
    key: &str,
    model: &Value,
    uploaded: Option<Value>,
) -> Result<Value> {
    let document = load(&*store.conn()?, project)?;
    let mut input =
        json!({"action":"prepare_generation","document":document,"taskKey":key,"model":model});
    if let Some(uploaded) = uploaded {
        input["uploaded"] = uploaded;
    }
    let value = runtime::execute(input)?;
    ensure!(
        value.get("error").is_none(),
        "{}",
        value["error"]
            .as_str()
            .unwrap_or("Generation preflight failed")
    );
    Ok(value)
}
pub fn sync_job(db: &Connection, job: &Value) -> Result<()> {
    if !job["shot"]["canvasGeneration"].is_object() {
        return Ok(());
    }
    let project = job["projectId"].as_str().context("Missing project")?;
    let before = load(db, project)?;
    let result = runtime::execute(json!({"action":"job","document":before,"job":job}))?;
    ensure!(
        result.get("error").is_none(),
        "{}",
        result["error"]
            .as_str()
            .unwrap_or("Job reconciliation failed")
    );
    if before != result["document"] {
        persist(db, &before, &result["document"])?;
    }
    Ok(())
}
pub use super::production_recovery::recover;
