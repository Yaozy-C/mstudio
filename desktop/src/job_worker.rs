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
    match job["status"].as_str()? {
        "IN_QUEUE" | "IN_PROGRESS" | "CANCEL_REQUESTED" | "UNKNOWN" | "SUBMITTING"
            if job["requestId"].as_str().is_some_and(|s| !s.is_empty()) =>
        {
            Some(Work::Status)
        }
        "COMPLETED" | "FAILED" | "CANCELLED" => {
            let count = job["outputCount"]
                .as_u64()
                .unwrap_or(u64::from(job["status"] == "COMPLETED"));
            // Empty completed output must surface an error rather than silently finish.
            let count = if job["status"] == "COMPLETED" {
                count.max(1)
            } else {
                count
            };
            (0..count as usize)
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
    let mut statuses = active.values().filter(|w| **w == Work::Status).count();
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
            Work::Status if statuses < 4 => statuses += 1,
            Work::Import(_) if imports < 2 => imports += 1,
            _ => continue,
        }
        result.push((id.to_owned(), work));
    }
    result
}
fn list(store: &Store) -> Result<Vec<Value>> {
    let db = store.db.lock().unwrap();
    let mut statement =
        db.prepare("SELECT json_remove(data,'$.input','$.result','$.outputs','$.shot') FROM jobs")?;
    let rows = statement.query_map([], |r| r.get::<_, String>(0))?;
    rows.map(|row| Ok(serde_json::from_str(&row?)?)).collect()
}
pub async fn reset(app: &tauri::AppHandle, id: &str) -> Result<()> {
    let _lock = crate::job_locks::acquire(id).await;
    let store = app.state::<Store>();
    let mut job = jobs::get(&store, id)?;
    job.as_object_mut().unwrap().remove("sync");
    jobs::save(&store, &job)?;
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
    let mut job = jobs::get(&store, id)?;
    update_sync(&mut job, stage, error, now());
    jobs::save(&store, &job)?;
    if let Some(project) = job["projectId"].as_str() {
        crate::project_service::notify(app, project);
    }
    Ok(())
}
fn update_sync(job: &mut Value, stage: Work, error: Option<anyhow::Error>, time: u64) {
    // A concurrent manual recovery may already have completed this operation.
    if error.is_some() && work(job) != Some(stage) {
        return;
    }
    let stage_name = if stage == Work::Status {
        "status"
    } else {
        "import"
    };
    if let Some(error) = error {
        let attempt = if job["sync"]["stage"] == stage_name {
            job["sync"]["attempt"].as_u64().unwrap_or(0)
        } else {
            0
        } + 1;
        let issue = crate::app_error::AppError::from_error(
            &error,
            if stage == Work::Status {
                "JOB_SYNC_FAILED"
            } else {
                "RESULT_IMPORT_FAILED"
            },
            stage_name,
        );
        job["sync"] = json!({"stage":stage_name,"attempt":attempt,"paused":attempt >= 4 || (!issue.retryable && !matches!(issue.code.as_str(), "JOB_SYNC_FAILED" | "RESULT_IMPORT_FAILED")),
            "nextAt":time + (4 * 2u64.pow(attempt.min(3) as u32)).min(30),"error":issue.to_string()});
    } else {
        job["sync"] = json!({"stage":stage_name,"attempt":0,"paused":false,
            "nextAt": if stage == Work::Status && work(job) == Some(Work::Status) { time + 4 } else { 0 }});
    }
}
pub fn start(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        if let Ok(jobs) = list(&app.state::<Store>()) {
            for mut candidate in jobs {
                if crate::model_adapters::recover_cancelled(&mut candidate)
                    && let Some(id) = candidate["id"].as_str()
                {
                    let _lock = crate::job_locks::acquire(id).await;
                    let store = app.state::<Store>();
                    if let Ok(mut job) = jobs::get(&store, id)
                        && crate::model_adapters::recover_cancelled(&mut job)
                    {
                        let _ = jobs::save(&store, &job);
                    }
                }
            }
        }
        let mut active = HashMap::new();
        let mut running: FuturesUnordered<BoxFuture<'static, String>> = FuturesUnordered::new();
        let mut interval = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = interval.tick() => {},
                Some(id) = running.next(), if !running.is_empty() => { active.remove(&id); },
            }
            let jobs = match list(&app.state::<Store>()) {
                Ok(jobs) => jobs,
                Err(error) => {
                    eprintln!("Job recovery scan failed: {error}");
                    continue;
                }
            };
            for (id, stage) in candidates(jobs, &active, now()) {
                active.insert(id.clone(), stage);
                let app = app.clone();
                running.push(Box::pin(async move {
                    let result = match stage {
                        Work::Status => jobs::refresh(&app, &id).await.map(|_| ()),
                        Work::Import(index) => crate::job_download::download(&app, &id, index)
                            .await
                            .map(|_| ()),
                    };
                    let _ = record(&app, &id, stage, result.err()).await;
                    id
                }));
            }
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn independent_pools_skip_slow_jobs_and_resume_durable_backoff() {
        let jobs: Vec<_> = (0..100)
            .map(|i| json!({"id":i.to_string(),"status":"IN_PROGRESS","requestId":"r"}))
            .collect();
        let mut active = HashMap::from([("0".into(), Work::Status)]);
        let next = candidates(jobs, &active, 100);
        assert_eq!(next.len(), 3);
        assert!(next.iter().all(|(id, _)| id != "0"));
        for (id, stage) in next {
            active.insert(id, stage);
        }
        let downloads = (0..100)
            .map(|i| json!({"id":format!("d{i}"),"status":"COMPLETED","outputCount":1}))
            .collect();
        assert_eq!(candidates(downloads, &active, 100).len(), 2);
        assert!(candidates(vec![json!({"id":"paused","status":"IN_PROGRESS","requestId":"r","sync":{"paused":true}}),
            json!({"id":"later","status":"IN_PROGRESS","requestId":"r","sync":{"nextAt":101}})], &HashMap::new(), 100).is_empty());
    }
    #[test]
    fn retry_backoff_and_pause_survive_serialization_and_success_resets_them() {
        let mut job = json!({"id":"job","status":"IN_PROGRESS","requestId":"r"});
        for attempt in 1..=4 {
            update_sync(
                &mut job,
                Work::Status,
                Some(anyhow::anyhow!("temporary network failure")),
                100,
            );
            job = serde_json::from_str(&job.to_string()).unwrap();
            assert_eq!(job["sync"]["attempt"], attempt);
            assert_eq!(job["sync"]["paused"], attempt == 4);
            assert!(candidates(vec![job.clone()], &HashMap::new(), 100).is_empty());
        }
        update_sync(&mut job, Work::Status, None, 200);
        assert_eq!(job["sync"]["attempt"], 0);
        assert_eq!(job["sync"]["nextAt"], 204);
        assert_eq!(candidates(vec![job], &HashMap::new(), 204).len(), 1);
    }
    #[test]
    fn partial_imports_resume_at_missing_output_and_unknown_submissions_are_never_replayed() {
        let job =
            json!({"status":"COMPLETED","outputCount":3,"assets":[{"id":"a"},null,{"id":"c"}]});
        assert_eq!(work(&job), Some(Work::Import(1)));
        assert_eq!(
            work(&json!({"status":"COMPLETED","outputCount":1,"asset":{"id":"a"}})),
            None
        );
        assert_eq!(work(&json!({"status":"UNKNOWN"})), None);
        assert_eq!(work(&json!({"status":"FAILED","outputCount":0})), None);
        assert_eq!(
            work(&json!({"status":"COMPLETED","outputCount":0})),
            Some(Work::Import(0))
        );
    }
}
