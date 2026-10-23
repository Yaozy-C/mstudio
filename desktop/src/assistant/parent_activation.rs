//! Resident root delivery, matching DSH's sendWaking/notifySettlement:
//! running receives at a step boundary, idle follows up, closing never wakes.
use super::{
    harness::{self, Host, ProjectHost, session::Session},
    pending, provider,
};
use crate::database::Store;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};
use tauri::{Emitter, Manager};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Running,
    Idle,
    Closing,
}
#[derive(Debug)]
struct Delivery {
    phase: Phase,
}
impl Delivery {
    fn notify(&mut self) -> bool {
        if self.phase == Phase::Idle {
            self.phase = Phase::Running;
            true
        } else {
            false
        }
    }
    fn idle(&mut self, pending: bool) -> bool {
        if self.phase == Phase::Closing {
            return false;
        }
        self.phase = Phase::Idle;
        if pending { self.notify() } else { false }
    }
}
struct Activation {
    host: ProjectHost,
    binding: Value,
    key: String,
    delivery: Mutex<Delivery>,
}
static ROOTS: LazyLock<Mutex<HashMap<String, Arc<Activation>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

pub fn register(host: &ProjectHost, binding: Value, key: &str) {
    let Some(tool) = &host.tool else {
        return;
    };
    let activation = Arc::new(Activation {
        host: host.clone(),
        binding,
        key: key.into(),
        delivery: Mutex::new(Delivery {
            phase: Phase::Running,
        }),
    });
    if let Some(old) = ROOTS
        .lock()
        .unwrap()
        .insert(tool.project.clone(), activation)
    {
        old.delivery.lock().unwrap().phase = Phase::Closing;
    }
}

pub fn close(project: &str) {
    if let Some(root) = ROOTS.lock().unwrap().remove(project) {
        root.delivery.lock().unwrap().phase = Phase::Closing;
    }
}
pub fn close_all() {
    for (_, root) in ROOTS.lock().unwrap().drain() {
        root.delivery.lock().unwrap().phase = Phase::Closing;
    }
}

pub(crate) fn fail(app: &tauri::AppHandle, project: &str, turn: &str, error: &str) {
    if let Some(root) = ROOTS.lock().unwrap().get(project)
        && root
            .host
            .tool
            .as_ref()
            .is_some_and(|tool| tool.turn == turn)
    {
        root.delivery.lock().unwrap().phase = Phase::Closing;
        root.host.token.cancel();
    }
    if let Err(save_error) = super::execution::fail_delivery(app, project, turn, error) {
        let error = format!("{error}; cannot persist execution failure: {save_error}");
        let _ = app.emit(
            "agent-progress",
            json!({"projectId":project,"turnId":turn,
            "kind":"turn/end","payload":{"status":"failed","error":error}}),
        );
    }
}

/// Called only after the settlement notification transaction has committed.
pub fn notify(project: &str) -> Result<(), String> {
    let root = ROOTS.lock().unwrap().get(project).cloned();
    if let Some(root) = root {
        if !undelivered(&root)? {
            return Ok(());
        }
        let wake = root.delivery.lock().unwrap().notify();
        if wake {
            spawn_followup(root);
        }
    }
    Ok(())
}

fn undelivered(root: &Activation) -> Result<bool, String> {
    let tool = root.host.tool.as_ref().unwrap();
    harness::mailbox::pending(
        &tool.app.state::<Store>().db.lock().unwrap(),
        &tool.project,
        &tool.profile.id,
        &tool.turn,
    )
}

/// The caller releases PendingTurn first, so followup can acquire execution.
/// Recheck the durable inbox under the delivery lock to close the final-step race.
pub fn settled(project: &str, turn: &str, completed: bool) -> Result<(), String> {
    let root = ROOTS.lock().unwrap().get(project).cloned();
    if let Some(root) = root {
        if root.host.tool.as_ref().unwrap().turn != turn {
            return Ok(());
        }
        let wake = (|| -> Result<bool, String> {
            let mut delivery = root.delivery.lock().unwrap();
            if !completed {
                delivery.phase = Phase::Closing;
                return Ok(false);
            }
            Ok(delivery.idle(undelivered(&root)?))
        })();
        let wake = match wake {
            Ok(wake) => wake,
            Err(error) => {
                let tool = root.host.tool.as_ref().unwrap();
                fail(&tool.app, project, turn, &error);
                return Err(error);
            }
        };
        if wake {
            spawn_followup(root);
        }
    }
    Ok(())
}

// Boxing breaks the recursive spawn/settlement future type. There is one
// runner per resident root; further notices remain in its committed inbox.
fn spawn_followup(root: Arc<Activation>) {
    let future: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> =
        Box::pin(async move {
            if let Err(error) = followup(&root).await {
                let tool = root.host.tool.as_ref().unwrap();
                fail(&tool.app, &tool.project, &tool.turn, &error);
            }
        });
    tauri::async_runtime::spawn(future);
}
async fn followup(root: &Arc<Activation>) -> Result<(), String> {
    let mut host = root.host.clone();
    let tool = host.tool.as_mut().unwrap();
    let project = tool.project.clone();
    let turn = tool.turn.clone();
    let pending = pending::PendingTurn::begin(&project).map_err(|e| e.to_string())?;
    {
        let delivery = root.delivery.lock().unwrap();
        if delivery.phase == Phase::Closing {
            return Ok(());
        }
    }
    tool.token = pending.token.clone();
    tool.deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(20 * 60);
    host.token = pending.token.clone();
    let app = tool.app.clone();
    let agent_id = tool.profile.id.clone();
    let store = app.state::<Store>();
    let mut messages = harness::session::restore(&store, &project, &turn, &root.binding)?
        .ok_or("Parent session unavailable")?;
    let tool = host.tool.as_mut().unwrap();
    tool.profile = super::profiles::resolve(&store.db.lock().unwrap(), Some(&tool.profile.id))
        .map_err(|e| e.to_string())?;
    super::turn_context::refresh(tool, &mut messages)?;
    // This is a continuation of the existing Agent, not a fabricated user request.
    reopen_response(&store, &project, &turn)?;
    super::execution::publish(&app, &project).map_err(|e| e.to_string())?;
    host.record(
        "turn/start",
        json!({"source":"subagent-settled","agentId":agent_id}),
    )?;
    let result = async {
        let model = provider::model(&host.media_profile, &root.key, Some(&store))?;
        harness::run(
            &model,
            &host.media_profile,
            &host,
            Session::new(messages),
            true,
            &root.key,
        )
        .await
    }
    .await;
    super::execution::finish(&app, &project, &turn, &result, &pending.token)
        .map_err(|e| e.to_string())?;
    drop(pending);
    settled(&project, &turn, result.is_ok())?;
    Ok(())
}

pub(super) fn reopen_response(store: &Store, project: &str, turn: &str) -> Result<(), String> {
    let changed = store.db.lock().unwrap().execute(
        "UPDATE agent_messages SET attribution=json_set(attribution,'$.status','running','$.error',NULL) WHERE project_id=?1 AND role='assistant' AND json_extract(attribution,'$.turnId')=?2 AND json_extract(attribution,'$.status') IN ('waiting','completed')",
        rusqlite::params![project,turn],
    ).map_err(|e|e.to_string())?;
    if changed != 1 {
        return Err("Parent response is no longer resumable".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "parent_activation_tests.rs"]
mod tests;
