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
        if let Err(_error) = production::recover(&app.state::<Store>()) {
            tracing::error!(event = "production_recovery_failed");
            return;
        }
        let mut active = HashSet::new();
        let mut running: FuturesUnordered<BoxFuture<'static, (String, String)>> =
            FuturesUnordered::new();
        let mut tick = tokio::time::interval(std::time::Duration::from_secs(1));
        loop {
            tokio::select! {
                _ = tick.tick() => {},
                Some(id) = running.next(), if !running.is_empty() => { active.remove(&id); },
            }
            let candidates = production::candidates(&app.state::<Store>().db.lock().unwrap());
            let Ok(candidates) = candidates else {
                continue;
            };
            for (project, key) in candidates {
                if active.len() >= 2 {
                    break;
                }
                if active.contains(&(project.clone(), key.clone())) {
                    continue;
                }
                let task = match production::claim(&app.state::<Store>(), &project, &key) {
                    Ok(Some(task)) => task,
                    _ => continue,
                };
                active.insert((project.clone(), key.clone()));
                project_service::notify(&app, &project);
                let worker = app.clone();
                running.push(Box::pin(async move {
                    if let Err(error) = submit(&worker, &project, &key, &task).await {
                        let store = worker.state::<Store>();
                        let submission = task["submissionId"].as_str().unwrap_or("");
                        // A reserved job is authoritative about a potentially paid effect.
                        // Never replace it with a local "failed, retry" interpretation.
                        if let Ok(job) = crate::jobs::get(&store, submission) {
                            if let Err(_error) =
                                production::sync_job(&store.db.lock().unwrap(), &job)
                            {
                                tracing::warn!(event = "task_reconciliation_failed");
                            }
                        } else {
                            let issue = crate::app_error::AppError::from_error(
                                &error,
                                "VALIDATION_FAILED",
                                "prepare",
                            );
                            let _ = production::patch(
                                &store,
                                &project,
                                &key,
                                submission,
                                &["UPLOADING", "SUBMITTING"],
                                json!({"status":"FAILED","error":issue.to_string()}),
                            );
                        }
                    }
                    project_service::notify(&worker, &project);
                    (project, key)
                }));
            }
        }
    });
}
