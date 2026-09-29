use super::*;

pub(super) async fn drive_child(
    child: &ChildHost,
    profile: &Profile,
    key: &str,
    messages: Vec<Message>,
    deadline: tokio::time::Instant,
) -> Result<String, String> {
    let model = provider::builder(profile, key)?
        .build()
        .model_handle()
        .clone();

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
        let update = (|| -> Result<(), String> {
            let mut db = store.db.lock().unwrap();
            let tx = db.transaction().map_err(|e| e.to_string())?;
            let changed = tx.execute("UPDATE subagent_runs SET status=?2,output=?3,notified=?4,updated=unixepoch() WHERE id=?1 AND last_turn=?5",rusqlite::params![id,status,output,notified as i32,tool.turn]).map_err(|e| e.to_string())?;
            if changed == 1 && !notified {
                tx.execute("INSERT INTO subagent_notices(child_id,project_id,parent_agent_id,status,output) SELECT id,project_id,parent_agent_id,?2,?3 FROM subagent_runs WHERE id=?1",rusqlite::params![id,status,output]).map_err(|e| e.to_string())?;
            }
            tx.commit().map_err(|e| e.to_string())
        })();
        if let Err(error) = update {
            eprintln!("subagent settlement failed: {error}");
        }
        let _ = journal::append(
            &store,
            &tool.project,
            &tool.turn,
            "subagent/settled",
            json!({"childId":id,"status":status,"stopReason":stop_reason(answer, child),"output":output}),
        );
    }
}
