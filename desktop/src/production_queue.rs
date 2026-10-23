use crate::{
    database::Store,
    project_service::{self, production},
};
use anyhow::{Context, Result, ensure};
use futures::{StreamExt, future::BoxFuture, stream::FuturesUnordered};
use serde_json::{Value, json};
use std::collections::HashSet;
use tauri::Manager;
async fn submit(app: &tauri::AppHandle, project: &str, key: &str, task: &Value) -> Result<()> {
    let store = app.state::<Store>();
    let submission = task["submissionId"]
        .as_str()
        .context("Missing submission identity")?;
    let model = crate::models::media::resolved(&store.db.lock().unwrap())?
        .into_iter()
        .find(|m| Some(m.id.as_str()) == task["modelId"].as_str() && m.enabled)
        .context("Selected model missing or disabled")?;
    let agent = crate::assistant::profiles::resolve(&store.db.lock().unwrap(), Some("production"))?;
    ensure!(
        agent.enabled && agent.tool_ids.contains(&"media-generation".into()),
        "Media generation permission required"
    );
    let model_value = serde_json::to_value(&model)?;
    let prepared = production::prepare(&store, project, key, &model_value, None)?;
    let refs: Vec<crate::reference_commands::Reference> =
        serde_json::from_value(prepared["references"].clone())?;
    let uploaded = if refs.is_empty() {
        json!([])
    } else {
        serde_json::to_value(
            crate::reference_commands::upload_references(
                app.clone(),
                project.into(),
                refs,
                true,
                model.id.clone(),
            )
            .await
            .map_err(anyhow::Error::msg)?,
        )?
    };
    let prepared = production::prepare(&store, project, key, &model_value, Some(uploaded))?;
    if !production::patch(
        &store,
        project,
        key,
        submission,
        &["UPLOADING"],
        json!({"jobId":submission,"status":"SUBMITTING","error":null}),
    )? {
        return Ok(());
    }
    project_service::notify(app, project);
    let mut shot = prepared["shot"].clone();
    shot["canvasGeneration"]["task"]["jobId"] = json!(submission);
    shot["canvasGeneration"]["task"]["status"] = json!("SUBMITTING");
    crate::jobs::submit_job(
        app.clone(),
        project.into(),
        model.endpoint,
        prepared["input"].clone(),
        shot,
        true,
        model.id,
        agent.id,
        agent.revision,
    )
    .await
    .map_err(anyhow::Error::msg)?;
    Ok(())
}
pub fn start(app: tauri::AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut running = FuturesUnordered::new();
        if let Err(error) = run(&app, &mut running).await {
            crate::jobs::runtime_health::report(&app, "queue", None, true, &error);
        }
        // Provider submissions already in flight must retain their response facts.
        while let Some(result) = running.next().await {
            if let Err(error) = result {
                crate::jobs::runtime_health::report(&app, "queue", None, true, &error);
            }
        }
    });
}
async fn run(
    app: &tauri::AppHandle,
    running: &mut FuturesUnordered<BoxFuture<'static, Result<(String, String)>>>,
) -> Result<()> {
    let recovery = app.clone();
    let report = tauri::async_runtime::spawn_blocking(move || {
        production::recover(&recovery.state::<Store>())
    })
    .await??;
    for project in report.changed {
        project_service::notify(app, &project);
    }
    for failure in report.deferred {
        if failure.job_id.is_some() {
            // Per-job diagnostics are already committed on the task and job.
            project_service::notify(app, &failure.project_id);
        } else {
            let error: crate::app_error::AppError = serde_json::from_str(&failure.error)?;
            crate::jobs::runtime_health::report(
                app,
                "startup",
                Some(&failure.project_id),
                false,
                &error.into(),
            );
        }
    }
    let mut active = HashSet::new();
    let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
    loop {
        tokio::select! {
            _ = tick.tick() => {},
            Some(result) = running.next(), if !running.is_empty() => { active.remove(&result?); },
        }
        let candidates = production::candidates(&*app.state::<Store>().conn()?)?;
        for (project, key) in candidates {
            if active.len() >= 2 {
                break;
            }
            if active.contains(&(project.clone(), key.clone())) {
                continue;
            }
            let Some(task) = production::claim(&app.state::<Store>(), &project, &key)? else {
                continue;
            };
            active.insert((project.clone(), key.clone()));
            project_service::notify(app, &project);
            let worker = app.clone();
            running.push(Box::pin(async move {
                if let Err(submission_error) = submit(&worker, &project, &key, &task).await {
                    let submission = task["submissionId"]
                        .as_str()
                        .context("Missing submission identity")?;
                    let _lock = crate::job_locks::acquire(submission).await;
                    if let Some(issue) = submission_failed(
                        &worker.state::<Store>(),
                        &project,
                        &key,
                        submission,
                        submission_error,
                    )? {
                        tracing::warn!(event="reserved_project_pending", project_id=%project, job_id=submission, code=%issue.code, stage=%issue.stage);
                    }
                }
                project_service::notify(&worker, &project);
                Ok((project, key))
            }));
        }
    }
}
fn submission_failed(
    store: &Store,
    project: &str,
    key: &str,
    submission: &str,
    error: anyhow::Error,
) -> Result<Option<crate::app_error::AppError>> {
    // A reserved job owns a potentially paid effect. Only local preflight failures
    // without a reservation may become retryable FAILED tasks.
    match crate::jobs::metadata(store, submission) {
        Ok(_) => match crate::jobs::reconcile(store, submission) {
            Ok(()) => Ok(None),
            Err(error) => match error.downcast_ref::<crate::app_error::AppError>() {
                // This error is returned only after the marker commits. Keep the
                // paid job isolated; the tracking worker owns its local recovery.
                Some(issue) if issue.code == "PROJECT_SYNC_FAILED" => Ok(Some(issue.clone())),
                _ => Err(error),
            },
        },
        Err(lookup_error)
            if matches!(
                lookup_error.downcast_ref::<rusqlite::Error>(),
                Some(rusqlite::Error::QueryReturnedNoRows)
            ) =>
        {
            let issue =
                crate::app_error::AppError::from_error(&error, "VALIDATION_FAILED", "prepare");
            production::patch(
                store,
                project,
                key,
                submission,
                &["UPLOADING", "SUBMITTING"],
                json!({"status":"FAILED","error":issue.to_string()}),
            )
            .map(|_| None)
            .map_err(|write_error| {
                anyhow::anyhow!("失败状态未保存：{write_error}；原提交错误：{issue}")
            })
        }
        Err(lookup_error) => Err(lookup_error.context(format!(
            "无法读取提交记录 {submission}；原提交错误：{error}"
        ))),
    }
}

#[cfg(test)]
#[path = "production_queue_tests.rs"]
mod tests;
