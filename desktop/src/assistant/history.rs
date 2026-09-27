use crate::database::Store;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
#[derive(Clone, Serialize, Deserialize)]
pub struct Message {
    pub id: i64,
    pub role: String,
    pub content: String,
    pub model: String,
    pub payload: Value,
    #[serde(default)]
    pub attribution: Option<Value>,
}
pub fn init(db: &rusqlite::Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS agent_messages(id INTEGER PRIMARY KEY AUTOINCREMENT,project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,role TEXT NOT NULL,content TEXT NOT NULL,model TEXT NOT NULL,payload TEXT NOT NULL,attribution TEXT); CREATE INDEX IF NOT EXISTS agent_messages_project ON agent_messages(project_id,id);")?;
    db.execute_batch("CREATE INDEX IF NOT EXISTS agent_messages_task ON agent_messages(project_id,json_extract(attribution,'$.agentId'),json_extract(attribution,'$.taskScope.taskId'),id);")?;
    Ok(())
}
pub fn read(store: &Store, project: &str) -> Result<Vec<Message>> {
    let db = store.db.lock().unwrap();
    let mut stmt=db.prepare("SELECT role,content,model,payload,attribution,id FROM (SELECT * FROM agent_messages WHERE project_id=?1 ORDER BY id DESC LIMIT 60) ORDER BY id")?;
    let rows = stmt
        .query_map([project], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, Option<String>>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter()
        .map(|(role, content, model, payload, attribution, id)| {
            let mut attribution: Option<Value> =
                attribution.map(|v| serde_json::from_str(&v)).transpose()?;
            if let Some(meta) = attribution.as_mut()
                && meta["createdAt"].is_null()
                && let Some(turn) = meta["turnId"].as_str()
            {
                let created: Option<i64> = db.query_row(
                    "SELECT MIN(created) FROM agent_events WHERE project_id=?1 AND turn_id=?2",
                    rusqlite::params![project, turn],
                    |r| r.get(0),
                )?;
                if let Some(created) = created {
                    meta["createdAt"] = serde_json::json!(created * 1000);
                }
            }
            Ok(Message {
                id,
                role,
                content,
                model,
                payload: serde_json::from_str(&payload)?,
                attribution,
            })
        })
        .collect()
}
#[cfg(test)]
pub fn append(
    store: &Store,
    project: &str,
    prompt: &str,
    payload: &Value,
    reply: &str,
    model: &str,
) -> Result<()> {
    append_attributed(store, project, prompt, payload, reply, model, None)
}
#[allow(clippy::too_many_arguments)]
#[cfg(test)]
pub fn append_attributed(
    store: &Store,
    project: &str,
    prompt: &str,
    payload: &Value,
    reply: &str,
    model: &str,
    attribution: Option<&Value>,
) -> Result<()> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    for (role, text, body) in [
        ("user", prompt, payload.clone()),
        ("assistant", reply, serde_json::json!(reply)),
    ] {
        tx.execute("INSERT INTO agent_messages(project_id,role,content,model,payload,attribution) VALUES(?1,?2,?3,?4,?5,?6)",rusqlite::params![project,role,text,model,body.to_string(),attribution.map(Value::to_string)])?;
    }
    tx.commit()?;
    Ok(())
}
pub fn page(
    store: &Store,
    project: &str,
    offset: usize,
    message_id: Option<i64>,
    text_offset: usize,
    task_id: Option<&str>,
) -> Result<Vec<Value>> {
    let db = store.db.lock().unwrap();
    let mut stmt=db.prepare("SELECT id,role,substr(content,?3,1500),length(content) FROM agent_messages WHERE project_id=?1 AND (?4 IS NULL OR id=?4) AND (?5 IS NULL OR json_extract(attribution,'$.taskScope.taskId')=?5) ORDER BY id DESC LIMIT 3 OFFSET ?2")?;
    Ok(stmt.query_map(rusqlite::params![project,offset as i64,text_offset as i64+1,message_id,task_id],|r|Ok(serde_json::json!({"id":r.get::<_,i64>(0)?,"role":r.get::<_,String>(1)?,"text":r.get::<_,String>(2)?,"nextTextOffset":if r.get::<_,usize>(3)?>text_offset+1500{Some(text_offset+1500)}else{None}})))?.collect::<rusqlite::Result<Vec<_>>>()?)
}

/// Persist both sides before any model or attachment I/O. A crash leaves an
/// identifiable interrupted response instead of erasing the user's request.
pub fn begin(
    store: &Store,
    project: &str,
    turn: &str,
    request: &Value,
    model: &str,
    meta: &Value,
) -> Result<()> {
    ensure!(
        !turn.is_empty()
            && turn.len() <= 80
            && turn.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "运行标识无效"
    );
    let text = request["production"]["instruction"].as_str().unwrap_or("");
    ensure!(
        !text.trim().is_empty() && text.len() <= 24000,
        "消息需为 1–8000 个字符"
    );
    ensure!(
        request["production"]["projectId"] == project && request.to_string().len() <= 100_000,
        "消息上下文无效或过长"
    );
    ensure!(
        request["refs"].as_array().is_some_and(|r| r.len() <= 12),
        "附件记录无效"
    );
    let payload = serde_json::json!([{"type":"text","text":text,"attachments":request["refs"],"taskTarget":{"id":request["targetNodeId"]}}]);
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let exists: bool = tx.query_row("SELECT EXISTS(SELECT 1 FROM agent_messages WHERE project_id=?1 AND json_extract(attribution,'$.turnId')=?2) OR EXISTS(SELECT 1 FROM agent_events WHERE project_id=?1 AND turn_id=?2)",rusqlite::params![project,turn],|r|r.get(0))?;
    ensure!(!exists, "此运行已经提交，请重新发送");
    tx.execute("INSERT INTO agent_messages(project_id,role,content,model,payload,attribution) VALUES(?1,'user',?2,?3,?4,?5)",rusqlite::params![project,text,model,payload.to_string(),meta.to_string()])?;
    let mut reply_meta = meta.clone();
    reply_meta.as_object_mut().unwrap().remove("request");
    reply_meta["status"] = serde_json::json!("running");
    tx.execute("INSERT INTO agent_messages(project_id,role,content,model,payload,attribution) VALUES(?1,'assistant','',?2,'\"\"',?3)",rusqlite::params![project,model,reply_meta.to_string()])?;
    tx.commit()?;
    Ok(())
}

pub fn update_payload(store: &Store, project: &str, turn: &str, payload: &Value) -> Result<()> {
    let count = store.db.lock().unwrap().execute("UPDATE agent_messages SET payload=?3 WHERE project_id=?1 AND role='user' AND json_extract(attribution,'$.turnId')=?2",rusqlite::params![project,turn,payload.to_string()])?;
    ensure!(count == 1, "消息记录不存在");
    Ok(())
}

pub fn finish(
    store: &Store,
    project: &str,
    turn: &str,
    text: &str,
    status: &str,
    error: Option<&str>,
) -> Result<()> {
    ensure!(
        ["completed", "failed", "cancelled"].contains(&status),
        "回答状态无效"
    );
    let count = store.db.lock().unwrap().execute("UPDATE agent_messages SET content=?3,payload=?4,attribution=json_set(attribution,'$.status',?5,'$.error',?6) WHERE project_id=?1 AND role='assistant' AND json_extract(attribution,'$.turnId')=?2 AND json_extract(attribution,'$.status')='running'",rusqlite::params![project,turn,text,serde_json::json!(text).to_string(),status,error])?;
    ensure!(count == 1, "回答记录不存在或已结束");
    Ok(())
}

pub fn interrupt_pending(db: &rusqlite::Connection) -> Result<()> {
    db.execute("UPDATE agent_messages SET attribution=json_set(attribution,'$.status','interrupted','$.error','上次回答因应用退出而中断，可重试') WHERE role='assistant' AND json_extract(attribution,'$.status')='running'", [])?;
    Ok(())
}
