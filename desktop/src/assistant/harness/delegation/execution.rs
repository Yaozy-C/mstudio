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

pub(super) fn settle(child: &ChildHost, id: &str, answer: &Result<String, String>, notified: bool) {
    if let Some(tool) = &child.inner.tool {
        let store = tool.app.state::<Store>();
        let mode: String = store
            .db
            .lock()
            .unwrap()
            .query_row("SELECT mode FROM subagent_runs WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .unwrap_or_default();
        let status = if answer.is_ok() {
            if mode == "oneShot" {
                "completed"
            } else {
                "idle"
            }
        } else if mode == "continuable" && child.inner.token.is_cancelled() {
            "ready"
        } else {
            "failed"
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
                .unwrap_or_else(|e| json!({"error":e.to_string()}));
        let delivery = super::super::delivery::for_turn(
            &store,
            &tool.project,
            &tool.turn,
            &generation["generationTasks"],
        );
        let artifact = json!({"childId":id,"agentId":tool.profile.id,"status":status,
            "stopReason":stop_reason(answer, child),"output":output,
            "changes":changes,"savedWork":saved_work,"delivery":delivery,
            "generationTasks":generation["generationTasks"],"generationStateError":generation["error"],
            "outputRef":output_ref});
        let notice = json!({"outputRef":output_ref,"stopReason":artifact["stopReason"],
            "savedWork":saved_work,"delivery":delivery,"generationTasks":generation["generationTasks"],
            "answerPreview":output.chars().take(1200).collect::<String>(),
            "readWith":"mstudio_read_agent_result(id,turnId) returns the complete original output and write receipts; pass outputRef in artifacts to the next specialist."}).to_string();
        let update = (|| -> Result<bool, String> {
            let mut db = store.db.lock().unwrap();
            let tx = db.transaction().map_err(|e| e.to_string())?;
            let changed = tx.execute("UPDATE subagent_runs SET status=?2,output=?3,notified=?4,updated=unixepoch() WHERE id=?1 AND last_turn=?5",rusqlite::params![id,status,output,notified as i32,tool.turn]).map_err(|e| e.to_string())?;
            if changed == 1 {
                crate::assistant::journal::insert(
                    &tx,
                    &tool.project,
                    &tool.turn,
                    "subagent/settled",
                    artifact.clone(),
                )
                .map_err(|e| e.to_string())?;
                crate::database::event_retention::finish(&tx, &tool.project, &tool.turn)
                    .map_err(|e| e.to_string())?;
            }
            if changed == 1 && !notified {
                tx.execute("INSERT INTO subagent_notices(child_id,project_id,parent_agent_id,status,output) SELECT id,project_id,parent_agent_id,?2,?3 FROM subagent_runs WHERE id=?1",rusqlite::params![id,status,notice]).map_err(|e| e.to_string())?;
            }
            tx.commit().map_err(|e| e.to_string())?;
            Ok(changed == 1 && !notified)
        })();
        let notify_parent = matches!(update, Ok(true));
        if update.is_ok() {
            let _ = crate::assistant::child_activity::record(tool, "subagent/settled", &artifact);
        }
        if let Err(error) = update {
            eprintln!("subagent settlement failed: {error}");
        }
        if notify_parent {
            crate::assistant::parent_activation::notify(&tool.project);
        }
    }
}

pub(super) fn result(
    child: &ChildHost,
    spawned_id: &str,
    generation: &str,
    answer: Result<String, String>,
) -> Value {
    let changes = child.changes.lock().unwrap().clone();
    let mut result = json!({"ok":answer.is_ok(),"agentId":child.inner.tool.as_ref().unwrap().profile.id,
            "childId":spawned_id,"stopReason":stop_reason(&answer,child),
            "applied":!changes.is_empty(),"changes":changes,
            "savedWork":super::super::saved_work::summary(&changes),
            "scripts":child.scripts.lock().unwrap().clone(),
            "outputRef":super::super::agent_artifacts::reference(spawned_id,generation)});
    let tool = child.inner.tool.as_ref().unwrap();
    match crate::assistant::generation_context::outcome(
        &tool.app.state::<Store>(),
        &tool.project,
        generation,
    ) {
        Ok(value) => result["generationTasks"] = value["generationTasks"].clone(),
        Err(error) => result["generationStateError"] = json!(error.to_string()),
    }
    result["delivery"] = super::super::delivery::for_turn(
        &tool.app.state::<Store>(),
        &tool.project,
        generation,
        &result["generationTasks"],
    );
    match answer {
        Ok(text) => result["answer"] = json!(text),
        Err(error) => result["error"] = json!(error),
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    async fn all_child_modes_share_capacity_and_cancel_queued_work() {
        let token = CancellationToken::new();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
        let mut permits = Vec::new();
        for _ in 0..super::super::super::MAX_CONCURRENCY {
            permits.push(acquire(&token, deadline).await.unwrap());
        }
        let waiting = acquire(&token, deadline);
        tokio::pin!(waiting);
        assert!(futures::poll!(&mut waiting).is_pending());
        permits.pop();
        let next = waiting.await.unwrap();
        assert_eq!(SLOTS.available_permits(), 0);
        let cancelled = token.child_token();
        let waiting = acquire(&cancelled, deadline);
        tokio::pin!(waiting);
        assert!(futures::poll!(&mut waiting).is_pending());
        cancelled.cancel();
        assert!(waiting.await.is_err());
        drop(next);
        drop(permits);
        assert_eq!(
            SLOTS.available_permits(),
            super::super::super::MAX_CONCURRENCY
        );
        assert!(acquire(&token, tokio::time::Instant::now()).await.is_err());
    }
}
