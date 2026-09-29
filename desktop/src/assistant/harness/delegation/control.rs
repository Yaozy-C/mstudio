use super::*;

pub fn notices(tool: &crate::assistant::tools::ProjectTool) -> Result<Vec<Message>, String> {
    super::super::mailbox::notices(
        &tool.app.state::<Store>(),
        &tool.project,
        &tool.profile.id,
        &tool.turn,
    )
}

pub async fn execute_control(host: &ProjectHost, call: &ToolCall) -> Value {
    let result = match call.function.name.as_str() {
        "mstudio_list_agents" => list_agents(host, &call.function.arguments),
        "mstudio_interrupt_agent" => interrupt_agent(host, &call.function.arguments),
        "mstudio_send_message" => send_message(host, &call.function.arguments),
        _ => Err("Unknown subagent control tool".into()),
    };
    result.unwrap_or_else(|error| json!({"error":error,"code":"SUBAGENT_CONTROL_FAILED"}))
}
pub(super) fn require_continuable(mode: Option<&str>) -> Result<(), String> {
    match mode {
        Some("continuable") => Ok(()),
        Some(_) => Err("This oneShot child cannot continue. Delegate again with existing results; use mode=continuable for subsequent revisions.".into()),
        None => Err("Direct child not found; use a child session ID from mstudio_list_agents, not a role ID.".into()),
    }
}

fn authorized_child(
    host: &ProjectHost,
    id: &str,
) -> Result<(String, String, String, String), String> {
    let parent = host.tool.as_ref().ok_or("Subagent control unavailable")?;
    let store = parent.app.state::<Store>();
    let db = store.db.lock().unwrap();
    let mode: Option<String> = db
        .query_row(
            "SELECT mode FROM subagent_runs WHERE id=?1 AND project_id=?2 AND parent_agent_id=?3",
            rusqlite::params![id, parent.project, parent.profile.id],
            |r| r.get(0),
        )
        .optional()
        .map_err(|e| e.to_string())?;
    require_continuable(mode.as_deref())?;
    db.query_row("SELECT status,profile,model_id,last_turn FROM subagent_runs WHERE id=?1 AND project_id=?2 AND parent_agent_id=?3",rusqlite::params![id,parent.project,parent.profile.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(|e|e.to_string())
}
fn validate_list_scope(args: &Value) -> Result<(), String> {
    if args.get("scope").is_some_and(|scope| scope != "children") {
        return Err(
            "Only scope=children is supported; descendant enumeration is unavailable".into(),
        );
    }
    Ok(())
}
fn list_agents(host: &ProjectHost, args: &Value) -> Result<Value, String> {
    validate_list_scope(args)?;
    let parent = host.tool.as_ref().ok_or("Subagent control unavailable")?;
    let store = parent.app.state::<Store>();
    let db = store.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT id,agent_id,status,mode,updated FROM subagent_runs WHERE project_id=?1 AND parent_agent_id=?2 AND mode='continuable' ORDER BY created,id").map_err(|e|e.to_string())?;
    let children = stmt.query_map(rusqlite::params![parent.project,parent.profile.id],|r| Ok(json!({"id":r.get::<_,String>(0)?,"agentId":r.get::<_,String>(1)?,"status":r.get::<_,String>(2)?,"mode":r.get::<_,String>(3)?,"updated":r.get::<_,i64>(4)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    Ok(json!({"children":children}))
}
fn interrupt_agent(host: &ProjectHost, args: &Value) -> Result<Value, String> {
    let id = args["agentId"].as_str().ok_or("Missing child agent ID")?;
    authorized_child(host, id)?;
    if let Some(run) = active().lock().unwrap().get(id) {
        run.token.cancel();
    }
    Ok(json!({"ok":true,"agentId":id}))
}
fn send_message(host: &ProjectHost, args: &Value) -> Result<Value, String> {
    let id = args["agentId"].as_str().ok_or("Missing child agent ID")?;
    let message = args["message"]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.len() <= 8000)
        .ok_or("Message must be 1-8000 bytes")?;
    let (status, profile_json, model_id, last_turn) = authorized_child(host, id)?;
    let parent = host.tool.as_ref().ok_or("Subagent control unavailable")?;
    let store = parent.app.state::<Store>();
    let message_id = {
        let db = store.db.lock().unwrap();
        db.execute(
            "INSERT INTO subagent_inbox(child_id,text,source) VALUES(?1,?2,?3)",
            rusqlite::params![
                id,
                message,
                super::super::mailbox::agent_source(&parent.profile.id, &parent.turn).to_string()
            ],
        )
        .map_err(|e| e.to_string())?;
        db.last_insert_rowid()
    };
    if status != "running" {
        start_continuation(parent, id, &profile_json, &model_id, &last_turn)?;
    }
    Ok(json!({"ok":true,"agentId":id,"messageId":message_id}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn list_contract_accepts_direct_children_and_rejects_unsupported_scopes() {
        let definition = control_definitions()
            .into_iter()
            .find(|tool| tool.name == "mstudio_list_agents")
            .unwrap();
        for args in [json!({}), json!({"scope":"children"})] {
            assert!(super::super::super::schema::validate(&definition.parameters, &args).is_ok());
            assert!(validate_list_scope(&args).is_ok());
        }
        // Stale model requests must fail, not silently return an incomplete list.
        for scope in [json!("descendants"), json!("all"), Value::Null, json!(1)] {
            let args = json!({"scope":scope});
            assert!(super::super::super::schema::validate(&definition.parameters, &args).is_err());
            assert!(validate_list_scope(&args).is_err());
        }
    }
}
