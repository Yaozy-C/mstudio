use super::*;
use crate::app_error::AppError;
use rusqlite::Connection;

pub(crate) fn issue(error: &anyhow::Error) -> AppError {
    let mut issue = AppError::new("PROJECT_SYNC_FAILED", "reconcile", &error.to_string());
    issue.retryable = true;
    issue
}

pub(super) fn apply(db: &Connection, job: &mut Value) -> Result<Option<AppError>> {
    project_write(db, job, crate::project_service::production::sync_job)
}
fn project_write(
    db: &Connection,
    job: &mut Value,
    update: impl FnOnce(&Connection, &Value) -> Result<Option<AppError>>,
) -> Result<Option<AppError>> {
    db.execute_batch("SAVEPOINT reconcile_job")?;
    let result = update(db, job);
    if result.is_err() {
        db.execute_batch("ROLLBACK TO reconcile_job")?;
    }
    db.execute_batch("RELEASE reconcile_job")?;
    let failure = match result {
        Ok(failure) => failure,
        Err(error) => Some(issue(&error)),
    };
    if let Some(issue) = failure {
        job["reconciliation"] = json!({"error":issue.to_string()});
        tracing::warn!(event = "job_reconciliation_failed", job_id = job["id"].as_str(),
            project_id = job["projectId"].as_str(), code = %issue.code, stage = %issue.stage);
        return Ok(Some(issue));
    }
    Ok(None)
}

// Lifecycle updates read metadata without loading model inputs or media payloads.
pub(crate) fn metadata(store: &Store, id: &str) -> Result<Value> {
    read_metadata(&*store.conn()?, id)
}
fn read_metadata(db: &Connection, id: &str) -> Result<Value> {
    let raw: String = db.query_row(
        "SELECT json_remove(data,'$.input','$.result','$.outputs') FROM jobs WHERE id=?1",
        [id],
        |r| r.get(0),
    )?;
    Ok(serde_json::from_str(&raw)?)
}
pub(crate) fn record(db: &Connection, id: &str, error: Option<&str>) -> Result<()> {
    let changed = if let Some(error) = error {
        db.execute(
            "UPDATE jobs SET data=json_set(data,'$.reconciliation',json(?2)) WHERE id=?1",
            rusqlite::params![id, json!({"error":error}).to_string()],
        )?
    } else {
        db.execute("UPDATE jobs SET data=CASE WHEN json_extract(data,'$.sync.stage')='reconcile' THEN json_remove(data,'$.reconciliation','$.sync') ELSE json_remove(data,'$.reconciliation') END WHERE id=?1", [id])?
    };
    ensure!(changed == 1, "任务已删除");
    Ok(())
}
// Err means tracking was not committed; Ok(Some) retains a durable project diagnostic.
pub(crate) fn save_tracking(
    store: &Store,
    id: &str,
    sync: Option<&Value>,
) -> Result<Option<AppError>> {
    let mut db = store.conn()?;
    let tx = db.transaction()?;
    let changed = if let Some(sync) = sync {
        tx.execute(
            "UPDATE jobs SET data=json_set(data,'$.sync',json(?2)) WHERE id=?1",
            rusqlite::params![id, sync.to_string()],
        )?
    } else {
        tx.execute(
            "UPDATE jobs SET data=json_remove(data,'$.sync') WHERE id=?1",
            [id],
        )?
    };
    ensure!(changed == 1, "任务已删除");
    let mut job = read_metadata(&tx, id)?;
    let error = project_write(&tx, &mut job, |db, job| {
        crate::project_service::production::sync_tracking(db, job).map(|_| None)
    })?;
    if let Some(error) = &error {
        record(&tx, id, Some(&error.to_string()))?;
    }
    tx.commit()?;
    if let Some(project) = job["projectId"].as_str() {
        store.project_changed(project);
    }
    Ok(error)
}
pub(crate) fn reconcile(store: &Store, id: &str) -> Result<()> {
    let mut db = store.conn()?;
    let tx = db.transaction()?;
    let mut job = read_metadata(&tx, id)?;
    // All callers hold the per-job lock, while the transaction protects project
    // changes from UI saves and other jobs.
    let error = apply(&tx, &mut job)?;
    record(&tx, id, error.as_ref().map(|e| e.to_string()).as_deref())?;
    tx.commit()?;
    if let Some(project) = job["projectId"].as_str() {
        store.project_changed(project);
    }
    if let Some(error) = error {
        return Err(error.into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "reconciliation_tests.rs"]
mod tests;
