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
        "mstudio_list_agents" => list_agents(host),
        "mstudio_interrupt_agent" => interrupt_agent(host, &call.function.arguments),
        "mstudio_send_message" => send_message(host, &call.function.arguments),
        _ => Err("未知子 Agent 控制工具".into()),
    };
    result.unwrap_or_else(|error| json!({"error":error,"code":"SUBAGENT_CONTROL_FAILED"}))
}
pub(super) fn require_continuable(mode: Option<&str>) -> Result<(), String> {
    match mode {
        Some("continuable") => Ok(()),
        Some(_) => Err("该子会话为 oneShot，不能续接。请重新委派并附上已有结果；需要后续修订时设置 mode=continuable。".into()),
        None => Err("找不到直属子 Agent；请用 mstudio_list_agents 返回的子会话 ID，不能使用角色 ID。".into()),
    }
}

fn authorized_child(
    host: &ProjectHost,
    id: &str,
) -> Result<(String, String, String, String), String> {
    let parent = host.tool.as_ref().ok_or("子 Agent 控制不可用")?;
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
fn list_agents(host: &ProjectHost) -> Result<Value, String> {
    let parent = host.tool.as_ref().ok_or("子 Agent 控制不可用")?;
    let store = parent.app.state::<Store>();
    let db = store.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT id,agent_id,status,mode,updated FROM subagent_runs WHERE project_id=?1 AND parent_agent_id=?2 AND mode='continuable' ORDER BY created,id").map_err(|e|e.to_string())?;
    let children = stmt.query_map(rusqlite::params![parent.project,parent.profile.id],|r| Ok(json!({"id":r.get::<_,String>(0)?,"agentId":r.get::<_,String>(1)?,"status":r.get::<_,String>(2)?,"mode":r.get::<_,String>(3)?,"updated":r.get::<_,i64>(4)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    Ok(json!({"children":children}))
}
fn interrupt_agent(host: &ProjectHost, args: &Value) -> Result<Value, String> {
    let id = args["agentId"].as_str().ok_or("缺少子 Agent ID")?;
    authorized_child(host, id)?;
    if let Some(run) = active().lock().unwrap().get(id) {
        run.token.cancel();
    }
    Ok(json!({"ok":true,"agentId":id}))
}
fn send_message(host: &ProjectHost, args: &Value) -> Result<Value, String> {
    let id = args["agentId"].as_str().ok_or("缺少子 Agent ID")?;
    let message = args["message"]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.len() <= 8000)
        .ok_or("消息需为 1–8000 字节")?;
    let (status, profile_json, model_id, last_turn) = authorized_child(host, id)?;
    let parent = host.tool.as_ref().ok_or("子 Agent 控制不可用")?;
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
