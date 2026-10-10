//! Resident root delivery, matching DSH's sendWaking/notifySettlement:
//! running receives at a step boundary, idle follows up, closing never wakes.
use super::{
    harness::{self, Host, ProjectHost, session::Session},
    history, pending, provider,
};
use crate::database::Store;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Arc, LazyLock, Mutex},
};
use tauri::Manager;

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

/// Called only after the settlement notification transaction has committed.
pub fn notify(project: &str, parent_turn: &str) {
    let root = ROOTS.lock().unwrap().get(project).cloned();
    if let Some(root) = root {
        let tool = root.host.tool.as_ref().unwrap();
        if tool.turn != parent_turn {
            return;
        }
        let wake = root.delivery.lock().unwrap().notify();
        if wake {
            spawn_followup(root);
        }
    }
}

fn undelivered(root: &Activation) -> Result<bool, String> {
    let tool = root.host.tool.as_ref().unwrap();
    tool.app.state::<Store>().db.lock().unwrap().query_row(
        "SELECT EXISTS(SELECT 1 FROM subagent_notices n JOIN subagent_runs r ON r.id=n.child_id WHERE n.project_id=?1 AND r.parent_turn=?2 AND n.delivered=0)",
        rusqlite::params![tool.project, tool.turn], |r| r.get(0),
    ).map_err(|e|e.to_string())
}

/// The caller releases PendingTurn first, so followup can acquire execution.
/// Recheck the durable inbox under the delivery lock to close the final-step race.
pub fn settled(project: &str, turn: &str, completed: bool) {
    let root = ROOTS.lock().unwrap().get(project).cloned();
    if let Some(root) = root {
        if root.host.tool.as_ref().unwrap().turn != turn {
            return;
        }
        let wake = {
            let mut delivery = root.delivery.lock().unwrap();
            if !completed {
                delivery.phase = Phase::Closing;
            }
            match undelivered(&root) {
                Ok(pending) => delivery.idle(pending),
                Err(error) => {
                    eprintln!("parent inbox read failed: {error}");
                    false
                }
            }
        };
        if wake {
            spawn_followup(root);
        }
    }
}

// Boxing breaks the recursive spawn/settlement future type. There is one
// runner per resident root; further notices remain in its committed inbox.
fn spawn_followup(root: Arc<Activation>) {
    let future: std::pin::Pin<Box<dyn std::future::Future<Output = ()> + Send>> =
        Box::pin(async move {
            if let Err(error) = followup(&root).await {
                root.delivery.lock().unwrap().phase = Phase::Closing;
                eprintln!("parent followup failed: {error}");
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
    if let Some(route) = &mut host.delegation {
        route.deadline = tool.deadline;
    }
    let app = tool.app.clone();
    let agent_id = tool.profile.id.clone();
    let store = app.state::<Store>();
    let messages = harness::session::restore(&store, &project, &turn, &root.binding)?
        .ok_or("Parent session unavailable")?;
    // This is a continuation of the existing Agent, not a fabricated user request.
    reopen_response(&store, &project, &turn)?;
    host.record(
        "turn/start",
        json!({"source":"subagent-settled","agentId":agent_id}),
    )?;
    let result = async {
        let model = provider::builder(&host.media_profile, &root.key)?
            .build()
            .model_handle()
            .clone();
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
    let status = if result.is_ok() {
        "completed"
    } else if pending.token.is_cancelled() {
        "cancelled"
    } else {
        "failed"
    };
    let partial = harness::session::partial(&store, &project, &turn);
    history::finish(
        &store,
        &project,
        &turn,
        result.as_deref().unwrap_or(&partial),
        status,
        result.as_ref().err().map(String::as_str),
    )
    .map_err(|e| e.to_string())?;
    host.record(
        "turn/end",
        json!({"status":status,"error":result.as_ref().err()}),
    )?;
    drop(pending);
    settled(&project, &turn, result.is_ok());
    Ok(())
}

fn reopen_response(store: &Store, project: &str, turn: &str) -> Result<(), String> {
    let changed = store.db.lock().unwrap().execute(
        "UPDATE agent_messages SET attribution=json_set(attribution,'$.status','running','$.error',NULL) WHERE project_id=?1 AND role='assistant' AND json_extract(attribution,'$.turnId')=?2 AND json_extract(attribution,'$.status')='completed'",
        rusqlite::params![project,turn],
    ).map_err(|e|e.to_string())?;
    if changed != 1 {
        return Err("Parent response is no longer resumable".into());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn idle_completion_wakes_once_and_running_completion_is_steered() {
        let mut d = Delivery { phase: Phase::Idle };
        assert!(d.notify());
        assert!(!d.notify());
        assert_eq!(d.phase, Phase::Running);
        // Already consumed by the running parent's next step: no extra turn.
        assert!(!d.idle(false));
        assert_eq!(d.phase, Phase::Idle);
    }
    #[test]
    fn completion_during_finalization_is_not_lost() {
        let mut d = Delivery {
            phase: Phase::Running,
        };
        assert!(!d.notify());
        assert!(d.idle(true));
        assert_eq!(d.phase, Phase::Running);
    }
    #[test]
    fn closing_parent_retains_notice_without_waking() {
        let mut d = Delivery {
            phase: Phase::Closing,
        };
        assert!(!d.notify());
        assert!(!d.idle(true));
        assert_eq!(d.phase, Phase::Closing);
    }
    #[test]
    fn idle_followup_reuses_saved_request_and_never_reopens_cancelled_response() {
        let (root, store, _) = super::super::attachment_tests::fixture();
        let request =
            json!({"refs":[],"production":{"projectId":"p","instruction":"Create a 15s ad"}});
        let meta = json!({"agentId":"coordinator","turnId":"parent","request":request});
        history::begin(&store, "p", "parent", &request, "model", &meta).unwrap();
        history::finish(
            &store,
            "p",
            "parent",
            "Creative task running",
            "completed",
            None,
        )
        .unwrap();
        reopen_response(&store, "p", "parent").unwrap();
        assert!(reopen_response(&store, "p", "parent").is_err());
        history::finish(&store, "p", "parent", "Script saved", "completed", None).unwrap();
        let history = history::read(&store, "p").unwrap();
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].content, "Create a 15s ad");
        assert_eq!(history[1].content, "Script saved");
        assert_eq!(history[0].attribution.as_ref().unwrap()["request"], request);
        reopen_response(&store, "p", "parent").unwrap();
        history::finish(&store, "p", "parent", "Stopped", "cancelled", None).unwrap();
        assert!(reopen_response(&store, "p", "parent").is_err());
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
