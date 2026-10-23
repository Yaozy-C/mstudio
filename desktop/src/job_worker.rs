//! App-owned recovery for submitted jobs. Never submits a new paid generation.
//! Status requests and downloads have separate rolling pools and durable backoff.
use crate::{database::Store, jobs};
use anyhow::Result;
use futures::{StreamExt, future::BoxFuture, stream::FuturesUnordered};
use serde_json::{Value, json};
use std::collections::HashMap;
use tauri::Manager;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Work {
    Reconcile,
    ValidateOutputs,
    Status,
    Import(usize),
}
fn now() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs()
}
fn work(job: &Value) -> Option<Work> {
    if job["reconciliation"]["error"].is_string() {
        return Some(Work::Reconcile);
    }
    match job["status"].as_str()? {
        "IN_QUEUE" | "IN_PROGRESS" | "CANCEL_REQUESTED" | "UNKNOWN" | "SUBMITTING"
            if job["requestId"].as_str().is_some_and(|s| !s.is_empty()) =>
        {
            Some(Work::Status)
        }
        "COMPLETED" | "FAILED" | "CANCELLED" => {
            let count = match jobs::outputs::count(job) {
                Ok(count) => count,
                Err(_) => return Some(Work::ValidateOutputs),
            };
            (0..count)
                .find(|&i| {
                    !job["assets"][i]["id"].is_string()
                        && !(i == 0 && job["asset"]["id"].is_string())
                })
                .map(Work::Import)
        }
        _ => None,
    }
}
fn candidates(jobs: Vec<Value>, active: &HashMap<String, Work>, time: u64) -> Vec<(String, Work)> {
    let mut jobs = jobs;
    // Oldest due jobs first: slow/failing jobs cannot starve the tail of a batch.
    jobs.sort_by_key(|j| {
        (
            j["sync"]["nextAt"].as_u64().unwrap_or(0),
            j["created"].as_u64().unwrap_or(0),
        )
    });
    let mut statuses = active
        .values()
        .filter(|w| !matches!(**w, Work::Import(_)))
        .count();
    let mut imports = active.len() - statuses;
    let mut result = Vec::new();
    for job in jobs {
        let Some(id) = job["id"].as_str() else {
            continue;
        };
        if active.contains_key(id)
            || job["sync"]["paused"] == true
            || job["sync"]["nextAt"].as_u64().unwrap_or(0) > time
        {
            continue;
        }
        let Some(work) = work(&job) else { continue };
        match work {
            Work::Status | Work::Reconcile | Work::ValidateOutputs if statuses < 4 => statuses += 1,
            Work::Import(_) if imports < 2 => imports += 1,
            _ => continue,
        }
        result.push((id.to_owned(), work));
    }
    result
}
fn list(store: &Store) -> Result<Vec<Value>> {
    let db = store.conn()?;
    let mut statement =
        db.prepare("SELECT json_remove(data,'$.input','$.result','$.outputs','$.shot') FROM jobs")?;
    let rows = statement.query_map([], |r| r.get::<_, String>(0))?;
    rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
}
pub async fn reset(app: &tauri::AppHandle, id: &str) -> Result<()> {
    let _lock = crate::job_locks::acquire(id).await;
    let store = app.state::<Store>();
    let job = jobs::metadata(&store, id)?;
    if let Some(error) = jobs::save_tracking(&store, id, None)? {
        return Err(error.into());
    }
    if let Some(project) = job["projectId"].as_str() {
        crate::project_service::notify(app, project);
    }
    Ok(())
}
async fn record(
    app: &tauri::AppHandle,
    id: &str,
    stage: Work,
    error: Option<anyhow::Error>,
) -> Result<()> {
    let _lock = crate::job_locks::acquire(id).await;
    let store = app.state::<Store>();
    if let Some(issue) = record_saved(&store, id, stage, error)? {
        // The marker and backoff are committed; this job stays isolated and reconcilable.
        tracing::warn!(event="tracking_saved_project_pending", job_id=id, code=%issue.code, stage=%issue.stage);
    }
    let job = jobs::metadata(&store, id)?;
    if let Some(project) = job["projectId"].as_str() {
        crate::project_service::notify(app, project);
    }
    Ok(())
}
fn record_saved(
    store: &Store,
    id: &str,
    stage: Work,
    error: Option<anyhow::Error>,
) -> Result<Option<crate::app_error::AppError>> {
    let mut job = jobs::metadata(store, id)?;
    if !update_sync(&mut job, stage, error, now()) {
        return Ok(None);
    }
    jobs::save_tracking(store, id, Some(&job["sync"]))
}
fn update_sync(job: &mut Value, stage: Work, error: Option<anyhow::Error>, time: u64) -> bool {
    let (stage_name, code) = match stage {
        Work::Status => ("status", "JOB_SYNC_FAILED"),
        Work::Import(_) => ("import", "RESULT_IMPORT_FAILED"),
        Work::Reconcile => ("reconcile", "PROJECT_SYNC_FAILED"),
        Work::ValidateOutputs => ("outputs", "VALIDATION_FAILED"),
    };
    // A concurrent manual recovery may already have completed this operation,
    // or another stage may own a newer failure and its backoff.
    if work(job) != Some(stage)
        && (error.is_some()
            || (job["sync"]["error"].is_string() && job["sync"]["stage"] != stage_name))
    {
        return false;
    }
    if let Some(error) = error {
        let attempt = if job["sync"]["stage"] == stage_name {
            job["sync"]["attempt"].as_u64().unwrap_or(0)
        } else {
            0
        } + 1;
        let issue = crate::app_error::AppError::from_error(&error, code, stage_name);
        job["sync"] = json!({"stage":stage_name,"attempt":attempt,"paused":attempt >= 4 || (!issue.retryable && !matches!(issue.code.as_str(), "JOB_SYNC_FAILED" | "RESULT_IMPORT_FAILED")),
            "nextAt":time + (4 * 2u64.pow(attempt.min(3) as u32)).min(30),"error":issue.to_string()});
    } else {
        job["sync"] = json!({"stage":stage_name,"attempt":0,"paused":false,
            "nextAt": if stage == Work::Status && work(job) == Some(Work::Status) { time + 4 } else { 0 }});
    }
    true
}
pub fn start(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut running = FuturesUnordered::new();
        if let Err(error) = run(&app, &mut running).await {
            jobs::runtime_health::report(&app, "tracking", None, true, &error);
        }
        // Finish already-started work without dispatching anything new.
        while let Some(result) = running.next().await {
            if let Err(error) = result {
                jobs::runtime_health::report(&app, "tracking", None, true, &error);
            }
        }
    });
}
async fn run(
    app: &tauri::AppHandle,
    running: &mut FuturesUnordered<BoxFuture<'static, Result<String>>>,
) -> Result<()> {
    let mut active = HashMap::new();
    let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
    loop {
        tokio::select! {
            _ = interval.tick() => {},
            Some(result) = running.next(), if !running.is_empty() => { active.remove(&result?); },
        }
        let jobs = list(&app.state::<Store>())?;
        for (id, stage) in candidates(jobs, &active, now()) {
            active.insert(id.clone(), stage);
            let app = app.clone();
            running.push(Box::pin(async move {
                let result = match stage {
                    Work::Status => jobs::refresh(&app, &id).await.map(|_| ()),
                    Work::Reconcile => {
                        let _lock = crate::job_locks::acquire(&id).await;
                        jobs::reconcile(&app.state::<Store>(), &id)
                    }
                    Work::Import(index) => crate::job_download::download(&app, &id, index)
                        .await
                        .map(|_| ()),
                    Work::ValidateOutputs => {
                        jobs::outputs::count(&jobs::metadata(&app.state::<Store>(), &id)?)
                            .map(|_| ())
                    }
                };
                record(&app, &id, stage, result.err()).await?;
                Ok(id)
            }));
        }
    }
}

#[cfg(test)]
#[path = "job_worker_tests.rs"]
mod tests;
