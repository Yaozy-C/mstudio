use super::*;

pub(super) async fn drive_child(
    child: &ChildHost,
    profile: &Profile,
    key: &str,
    messages: Vec<Message>,
    deadline: tokio::time::Instant,
) -> Result<String, String> {
    let model = provider::model(profile, key)?;

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
        let artifact = json!({"childId":id,"agentId":tool.profile.id,"status":status,
            "stopReason":stop_reason(answer, child),"output":output,
            "changes":child.changes.lock().unwrap().clone(),
            "outputRef":output_ref});
        let notice = json!({"outputRef":output_ref,"stopReason":artifact["stopReason"],
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
            let parent_turn: Option<String> = store
                .db
                .lock()
                .unwrap()
                .query_row(
                    "SELECT parent_turn FROM subagent_runs WHERE id=?1",
                    [id],
                    |r| r.get(0),
                )
                .ok();
            if let Some(parent_turn) = parent_turn {
                crate::assistant::parent_activation::notify(&tool.project, &parent_turn);
            }
        }
    }
}
