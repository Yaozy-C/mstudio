//! Failures of this app's background runners. Reporting never retries or submits work.
use crate::app_error::AppError;
use serde::Serialize;
use std::sync::Mutex;
use tauri::{Emitter, Manager};

#[derive(Default)]
pub struct Runtime(Mutex<Vec<Issue>>);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Issue {
    pub runner: String,
    pub project_id: Option<String>,
    pub stopped: bool,
    pub error: String,
}

pub fn report(
    app: &tauri::AppHandle,
    runner: &str,
    project: Option<&str>,
    stopped: bool,
    error: &anyhow::Error,
) {
    let issue = AppError::from_error(error, "JOB_SYNC_FAILED", runner);
    tracing::error!(event = "generation_runtime_failed", runner, project_id = project,
        stopped, code = %issue.code, stage = %issue.stage);
    let runtime = app.state::<Runtime>();
    let mut issues = runtime.0.lock().unwrap();
    let error = issue.to_string();
    if !issues
        .iter()
        .any(|i| i.runner == runner && i.project_id.as_deref() == project && i.error == error)
    {
        issues.push(Issue {
            runner: runner.into(),
            project_id: project.map(str::to_owned),
            stopped,
            error,
        });
    }
    drop(issues);
    let _ = app.emit("generation-runtime-changed", ());
}

#[tauri::command]
pub fn generation_runtime_issues(runtime: tauri::State<Runtime>) -> Vec<Issue> {
    runtime.0.lock().unwrap().clone()
}
