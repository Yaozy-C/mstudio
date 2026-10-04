//! A wait observes committed task state and sleeps on project events. It never
//! polls a provider, submits generation or consumes a model turn while waiting.
use super::{load, runtime};
use crate::database::Store;
use anyhow::{Result, ensure};
use rusqlite::Connection;
use serde_json::{Value, json};
use tokio::sync::broadcast::error::RecvError;
use tokio_util::sync::CancellationToken;

pub fn snapshot(db: &Connection, project: &str, keys: &[String]) -> Result<Value> {
    let document = load(db, project)?;
    let value = runtime::execute(
        json!({"action":"generation_outcomes","document":document,"taskKeys":keys}),
    )?;
    ensure!(
        value["generationTasks"].is_array(),
        "Cannot read generation task outcomes"
    );
    Ok(value)
}

pub async fn wait(
    store: &Store,
    project: &str,
    keys: &[String],
    token: &CancellationToken,
    deadline: tokio::time::Instant,
) -> Result<Value> {
    ensure!(
        !keys.is_empty() && keys.len() <= 30,
        "Wait needs 1–30 task keys"
    );
    // Subscribe before reading: a commit between the read and await stays queued.
    let mut events = store.project_events.subscribe();
    loop {
        let mut value = snapshot(&store.db.lock().unwrap(), project, keys)?;
        if token.is_cancelled() {
            value["waitEnded"] = json!("cancelled");
            return Ok(value);
        }
        if value["generationTasks"].as_array().is_some_and(|tasks| {
            tasks
                .iter()
                .any(|t| t["continuation"]["state"] != "waiting_service")
        }) {
            return Ok(value);
        }
        loop {
            tokio::select! {
                _ = token.cancelled() => {
                    value["waitEnded"] = json!("cancelled");
                    return Ok(value);
                },
                _ = tokio::time::sleep_until(deadline) => {
                    value["waitEnded"] = json!("deadline");
                    return Ok(value);
                },
                event = events.recv() => match event {
                    Ok(id) if id == project => break,
                    Err(RecvError::Lagged(_)) => break,
                    Err(RecvError::Closed) => anyhow::bail!("Project event stream closed"),
                    _ => {},
                },
            }
        }
    }
}

#[cfg(test)]
#[path = "generation_wait_tests.rs"]
mod tests;
