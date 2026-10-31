//! Decode only reconciliation metadata. Domain work runs without the database lock;
//! a short compare-and-swap transaction protects concurrent UI and worker changes.
use super::{domain, load, persist};
use crate::database::Store;
use anyhow::{Context, Result, ensure};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::collections::HashSet;

#[derive(Debug, Default)]
pub struct RecoveryReport {
    pub changed: Vec<String>,
    pub deferred: Vec<RecoveryFailure>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryFailure {
    pub project_id: String,
    pub job_id: Option<String>,
    pub error: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(super) struct JobFailure {
    job_id: String,
    error: String,
}

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

fn reconcile(before: &Snapshot) -> Result<(Value, Vec<JobFailure>)> {
    let reserved: HashSet<&str> = before.jobs.iter().map(|(id, _)| id.as_str()).collect();
    let mut document = before.document.clone();
    if let Some(tasks) = document["production"]["drafts"].as_object_mut() {
        for task in tasks.values_mut() {
            if task["status"] == "UNKNOWN" {
                let details = task["error"].as_str().unwrap_or("未收到生成服务的提交确认");
                let error = serde_json::from_str::<crate::app_error::AppError>(details)
                    .map(anyhow::Error::new)
                    .unwrap_or_else(|_| anyhow::anyhow!("{details}"));
                crate::job_recovery::submission_failed(task, &error);
                task["trackingPaused"] = json!(false);
            }
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
        .map(|(id, raw)| {
            serde_json::from_str::<Value>(raw)
                .with_context(|| format!("Invalid recovery metadata for job {id}"))
        })
        .collect::<Result<Vec<_>>>()?
        .into_iter()
        .filter(|job| job["shot"]["canvasGeneration"].is_object())
        .collect::<Vec<_>>();
    if jobs.is_empty() {
        return Ok((document, Vec::new()));
    }
    let result = domain::execute(json!({"action":"recover_jobs","document":document,"jobs":jobs}))?;
    ensure!(
        result["document"].is_object() && result.get("error").is_none(),
        "Job recovery failed: {}",
        result["error"]
            .as_str()
            .unwrap_or("Invalid domain response")
    );
    let failures = serde_json::from_value(result["failures"].clone())
        .context("Invalid recovery diagnostics")?;
    Ok((result["document"].clone(), failures))
}

// None means the snapshot became stale; the caller retries from current state.
pub(super) fn commit(
    store: &Store,
    project: &str,
    before: &Snapshot,
    after: &Value,
    failures: &[JobFailure],
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
    for (id, _) in &before.jobs {
        let error = failures
            .iter()
            .find(|f| &f.job_id == id)
            .map(|f| f.error.as_str());
        crate::jobs::reconciliation::record(&tx, id, error)?;
    }
    tx.commit()?;
    if changed {
        store.project_changed(project);
    }
    Ok(Some(changed))
}
fn recover_project(store: &Store, project: &str) -> Result<(bool, Vec<JobFailure>)> {
    for _ in 0..3 {
        let Some(before) = snapshot(&*store.conn()?, project)? else {
            return Ok((false, Vec::new()));
        };
        let (after, failures) = reconcile(&before)?;
        if let Some(changed) = commit(store, project, &before, &after, &failures)? {
            return Ok((changed, failures));
        }
    }
    // Concurrent work is already authoritative; leave it for the next reconciliation.
    anyhow::bail!("Project changed during recovery")
}
pub fn recover(store: &Store) -> Result<RecoveryReport> {
    let started = std::time::Instant::now();
    let projects: Vec<String> = {
        let db = store.conn()?;
        let mut statement = db.prepare("SELECT id FROM projects WHERE EXISTS(SELECT 1 FROM json_each(document,'$.production.drafts'))")?;
        statement
            .query_map([], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?
    };
    let mut report = RecoveryReport::default();
    for project in &projects {
        match recover_project(store, project) {
            Ok((changed, failures)) => {
                if changed {
                    report.changed.push(project.clone());
                }
                report
                    .deferred
                    .extend(failures.into_iter().map(|f| RecoveryFailure {
                        project_id: project.clone(),
                        job_id: Some(f.job_id),
                        error: f.error,
                    }));
            }
            Err(error) => {
                let issue = crate::jobs::reconciliation_issue(&error).to_string();
                report.deferred.push(RecoveryFailure {
                    project_id: project.clone(),
                    job_id: None,
                    error: issue,
                });
            }
        }
    }
    tracing::info!(
        event = "startup_recovery_complete",
        duration_ms = started.elapsed().as_millis() as u64,
        project_count = projects.len(),
        changed_count = report.changed.len(),
        deferred_count = report.deferred.len()
    );
    for failure in &report.deferred {
        tracing::warn!(event = "project_reconciliation_deferred", project_id = %failure.project_id,
            job_id = failure.job_id.as_deref(), code = "PROJECT_SYNC_FAILED", stage = "reconcile");
    }
    Ok(report)
}
