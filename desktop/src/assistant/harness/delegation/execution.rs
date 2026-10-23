use super::*;

static SLOTS: tokio::sync::Semaphore =
    tokio::sync::Semaphore::const_new(super::super::MAX_CONCURRENCY);

async fn acquire(
    token: &CancellationToken,
    deadline: tokio::time::Instant,
) -> Result<tokio::sync::SemaphorePermit<'static>, String> {
    if tokio::time::Instant::now() >= deadline {
        return Err("等待子任务执行名额超时".into());
    }
    tokio::select! {
        biased;
        _ = token.cancelled() => Err(crate::app_error::cancelled()),
        _ = tokio::time::sleep_until(deadline) => Err("等待子任务执行名额超时".into()),
        permit = SLOTS.acquire() => permit.map_err(|e|e.to_string()),
    }
}

pub(super) async fn drive_child(
    child: &ChildHost,
    profile: &Profile,
    key: &str,
    messages: Vec<Message>,
    deadline: tokio::time::Instant,
) -> Result<String, String> {
    let _permit = acquire(child.token(), deadline).await?;
    let store = child
        .inner
        .tool
        .as_ref()
        .map(|tool| tool.app.state::<Store>());
    let model = provider::model(profile, key, store.as_deref())?;

    super::super::driver::run_until(
        &model,
        profile,
        child,
        Session::new(messages),
        false,
        key,
        deadline,
    )
    .await
    .map_err(|error| crate::assistant::model_feedback::error(&error))
}
pub(super) fn stop_reason(answer: &Result<String, String>, child: &ChildHost) -> String {
    if child.token().is_cancelled() {
        "aborted".into()
    } else if answer.is_ok() {
        "completed".into()
    } else {
        child
            .terminal_reason
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "error".into())
    }
}

pub(super) fn settle(
    child: &ChildHost,
    id: &str,
    answer: &Result<String, String>,
    notified: bool,
) -> Result<(), String> {
    let tool = child
        .inner
        .tool
        .as_ref()
        .ok_or("Subagent project context missing")?;
    let store = tool.app.state::<Store>();
    let mode: String = store
        .db
        .lock()
        .unwrap()
        .query_row("SELECT mode FROM subagent_runs WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .map_err(|error| error.to_string())?;
    let status = match (
        mode.as_str(),
        answer.is_ok(),
        child.inner.token.is_cancelled(),
    ) {
        ("oneShot", true, _) => "completed",
        ("continuable", true, _) => "idle",
        ("continuable", false, true) => "ready",
        ("oneShot" | "continuable", false, _) => "failed",
        _ => return Err("Invalid subagent mode".into()),
    };
    let output = answer
        .as_ref()
        .map(String::as_str)
        .unwrap_or_else(|e| e.as_str());
    let output_ref = super::super::agent_artifacts::reference(id, &tool.turn);
    let changes = child.changes.lock().unwrap().clone();
    let saved_work = super::super::saved_work::summary(&changes);
    let generation =
        crate::assistant::generation_context::outcome(&store, &tool.project, &tool.turn)
            .map_err(|error| error.to_string())?;
    let delivery = super::super::delivery::for_turn(
        &store,
        &tool.project,
        &tool.turn,
        &generation["generationTasks"],
    )?;
    let artifact = json!({"ok":answer.is_ok(),"applied":!changes.is_empty(),
            "childId":id,"agentId":tool.profile.id,"status":status,
            "stopReason":stop_reason(answer, child),"output":output,
            "changes":changes,"savedWork":saved_work,"delivery":delivery,
            "generationTasks":generation["generationTasks"],"generationStateError":generation["error"],
            "outputRef":output_ref});
    let (notify_parent, activity) = persist_settlement(
        &mut store.db.lock().unwrap(),
        &tool.project,
        &tool.turn,
        id,
        &artifact,
        notified,
    )?;
    if let Some(activity) = activity {
        use tauri::Emitter;
        let _ = tool.app.emit("agent-child-progress", activity);
    }
    if notify_parent {
        crate::assistant::parent_activation::notify(&tool.project)?;
    }

    Ok(())
}

fn persist_settlement(
    db: &mut rusqlite::Connection,
    project: &str,
    turn: &str,
    id: &str,
    artifact: &Value,
    notified: bool,
) -> Result<(bool, Option<Value>), String> {
    let mut persist = || -> anyhow::Result<(bool, Option<Value>)> {
        let tx = db.transaction()?;
        let changed = tx.execute("UPDATE subagent_runs SET status=?2,output=?3,notified=?4,updated=unixepoch() WHERE id=?1 AND last_turn=?5",rusqlite::params![id,artifact["status"].as_str().ok_or_else(||anyhow::anyhow!("Missing child status"))?,artifact["output"].as_str().ok_or_else(||anyhow::anyhow!("Missing child output"))?,notified as i32,turn])?;
        anyhow::ensure!(changed == 1, "Child execution is no longer current");
        journal::insert(&tx, project, turn, "subagent/settled", artifact.clone())?;
        crate::database::event_retention::finish(&tx, project, turn)?;
        if !notified {
            tx.execute("INSERT INTO subagent_notices(child_id,project_id,parent_agent_id,status,output) SELECT id,project_id,parent_agent_id,?2,?3 FROM subagent_runs WHERE id=?1",rusqlite::params![id,artifact["status"].as_str(),settlement_notice(artifact)])?;
        }
        let activity = crate::assistant::child_activity::update(
            &tx,
            project,
            turn,
            "subagent/settled",
            artifact,
        )?;
        tx.commit()?;
        Ok((!notified, activity))
    };
    persist().map_err(|error| error.to_string())
}

pub(super) fn finish(
    child: &ChildHost,
    id: &str,
    generation: &str,
    answer: &Result<String, String>,
) {
    let result = settle(child, id, answer, false).and_then(|()| wake_pending(child, id));
    remove_active(id, generation);
    if let Err(error) = result {
        let Some(tool) = &child.inner.tool else {
            return;
        };
        let store = tool.app.state::<Store>();
        let error = match persistence::fail(
            &mut store.db.lock().unwrap(),
            &tool.project,
            &tool.turn,
            id,
            &error,
        ) {
            Ok(activity) => {
                if let Some(activity) = activity {
                    use tauri::Emitter;
                    let _ = tool.app.emit("agent-child-progress", activity);
                }
                error
            }
            Err(save_error) => format!("{error}; cannot persist child failure: {save_error}"),
        };
        crate::assistant::parent_activation::fail(
            &tool.app,
            &tool.project,
            &child.parent_turn,
            &error,
        );
    }
}

fn settlement_notice(artifact: &Value) -> String {
    json!({"ok":artifact["ok"],"applied":artifact["applied"],
        "outputRef":artifact["outputRef"],"stopReason":artifact["stopReason"],
        "savedWork":artifact["savedWork"],"delivery":artifact["delivery"],
        "generationTasks":artifact["generationTasks"],"generationStateError":artifact["generationStateError"],
        "answerPreview":artifact["output"].as_str().unwrap_or_default().chars().take(1200).collect::<String>(),
        "readWith":"mstudio_read_agent_result(id,turnId) returns the complete original output and write receipts; pass outputRef in artifacts to the next specialist."}).to_string()
}

#[cfg(test)]
#[path = "execution_tests.rs"]
mod tests;
