//! Decode only reconciliation metadata. Domain work runs without the database lock;
//! a short compare-and-swap transaction protects concurrent UI and worker changes.
use super::{load, persist, runtime};
use crate::database::Store;
use anyhow::{Result, ensure};
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(PartialEq)]
pub(super) struct Snapshot {
    pub(super) document: Value,
    jobs: Vec<(String, String)>,
}
pub(super) fn snapshot(db: &Connection, project: &str) -> Result<Option<Snapshot>> {
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)",
        [project],
        |r| r.get(0),
    )?;
    if !exists {
        return Ok(None);
    }
    let mut statement = db.prepare("SELECT id,json_remove(data,'$.input','$.result','$.outputs') FROM jobs WHERE project_id=?1 ORDER BY rowid")?;
    let jobs = statement
        .query_map([project], |r| Ok((r.get(0)?, r.get(1)?)))?
        .collect::<rusqlite::Result<_>>()?;
    Ok(Some(Snapshot {
        document: load(db, project)?,
        jobs,
    }))
}

fn reconcile(before: &Snapshot) -> Result<Value> {
    let reserved: HashSet<&str> = before.jobs.iter().map(|(id, _)| id.as_str()).collect();
    let mut document = before.document.clone();
    if let Some(tasks) = document["production"]["drafts"].as_object_mut() {
        for task in tasks.values_mut() {
            if matches!(task["status"].as_str(), Some("UPLOADING" | "SUBMITTING"))
                && !task["submissionId"]
                    .as_str()
                    .is_some_and(|id| reserved.contains(id))
            {
                task["status"] = json!("READY");
            }
        }
    }
    let jobs = before
        .jobs
        .iter()
        .filter_map(|(_, raw)| {
            let job: Value = serde_json::from_str(raw).ok()?;
            job["shot"]["canvasGeneration"].is_object().then_some(job)
        })
        .collect::<Vec<_>>();
    if jobs.is_empty() {
        return Ok(document);
    }
    let result =
        runtime::execute(json!({"action":"recover_jobs","document":document,"jobs":jobs}))?;
    ensure!(
        result["document"].is_object() && result.get("error").is_none(),
        "Job recovery failed"
    );
    if result["failedJobs"].as_u64().unwrap_or(0) > 0 {
        tracing::warn!(
            event = "job_reconciliation_deferred",
            job_count = result["failedJobs"].as_u64()
        );
    }
    Ok(result["document"].clone())
}

// None means the snapshot became stale; the caller retries from current state.
pub(super) fn commit(
    store: &Store,
    project: &str,
    before: &Snapshot,
    after: &Value,
) -> Result<Option<bool>> {
    let mut db = store.conn()?;
    let tx = db.transaction()?;
    let current = snapshot(&tx, project)?;
    if current.as_ref() != Some(before) {
        return Ok(None);
    }
    let changed = before.document != *after;
    if changed {
        persist(&tx, &before.document, after)?;
    }
    tx.commit()?;
    if changed {
        store.project_changed(project);
    }
    Ok(Some(changed))
}
fn recover_project(store: &Store, project: &str) -> Result<bool> {
    for _ in 0..3 {
        let Some(before) = snapshot(&*store.conn()?, project)? else {
            return Ok(false);
        };
        let after = reconcile(&before)?;
        if let Some(changed) = commit(store, project, &before, &after)? {
            return Ok(changed);
        }
    }
    // Concurrent work is already authoritative; leave it for the next reconciliation.
    anyhow::bail!("Project changed during recovery")
}
pub fn recover(store: &Store) -> Result<Vec<String>> {
    let started = std::time::Instant::now();
    let projects: Vec<String> = {
        let db = store.conn()?;
        let mut statement = db.prepare("SELECT id FROM projects WHERE EXISTS(SELECT 1 FROM json_each(document,'$.production.drafts'))")?;
        statement
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?
    };
    let mut changed = Vec::new();
    let mut deferred = 0;
    for project in &projects {
        match recover_project(store, project) {
            Ok(true) => changed.push(project.clone()),
            Ok(false) => {}
            Err(_) => {
                deferred += 1;
                tracing::warn!(event = "project_reconciliation_deferred");
            }
        }
    }
    tracing::info!(
        event = "startup_recovery_complete",
        duration_ms = started.elapsed().as_millis() as u64,
        project_count = projects.len(),
        changed_count = changed.len(),
        deferred_count = deferred
    );
    Ok(changed)
}
