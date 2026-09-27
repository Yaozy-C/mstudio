//! The exact provider messages are private session events, never UI tool-card payloads.
use super::Host;
use crate::{assistant::journal, database::Store};
use rig_core::message::{AssistantContent, Message, ToolCall, ToolResultContent, UserContent};
use serde_json::{Value, json};

pub struct Session {
    pub edit_progress: super::progress::EditProgress,
    pub messages: Vec<Message>,
    pub text: String,
    pub usage_anchor: Option<super::budget::UsageAnchor>,
}
impl Session {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            edit_progress: Default::default(),
            messages,
            text: String::new(),
            usage_anchor: None,
        }
    }
    pub fn append(&mut self, host: &impl Host, message: Message) -> Result<(), String> {
        host.record("session/message", json!({"message":message}))?;
        self.messages.push(message);
        Ok(())
    }
    pub fn publish(&mut self, host: &impl Host, delta: &str) -> Result<(), String> {
        if self.text.len() + delta.len() > 100_000 {
            return Err("模型回答过长；已输出的内容保留".into());
        }
        self.text.push_str(delta);
        host.record("assistant/partial", json!({"text":self.text,"delta":delta}))
    }
    pub fn replace_range(
        &mut self,
        host: &impl Host,
        start: usize,
        end: usize,
        replacement: Message,
    ) -> Result<(), String> {
        if start >= end || end > self.messages.len() {
            return Err("压缩范围无效".into());
        }
        host.record(
            "session/compaction",
            json!({"start":start,"end":end,"message":replacement}),
        )?;
        self.messages.splice(start..end, [replacement]);
        // Keep provider calibration; pressure accounts for the changed raw size.
        Ok(())
    }
}
pub fn result_message(call: &ToolCall, value: &Value) -> Message {
    if let Some(image) = value.get("__offloadedImage")
        && let Ok(image) = serde_json::from_value::<rig_core::message::Image>(image.clone())
    {
        return Message::User {
            content: vec![UserContent::tool_result_for(
                call.id.clone(),
                call.provider.clone(),
                &call.function.name,
                vec![
                    ToolResultContent::text(format!(
                        "图像 {} 已重新加载。",
                        value["imageId"].as_str().unwrap_or("")
                    )),
                    ToolResultContent::Image(image),
                ],
            )],
        };
    }
    Message::User {
        content: vec![UserContent::tool_result_for(
            call.id.clone(),
            call.provider.clone(),
            &call.function.name,
            vec![ToolResultContent::text(value.to_string())],
        )],
    }
}

/// Detect a project snapshot so an unchanged snapshot is not injected twice.
pub fn project_snapshot_text(message: &Message) -> Option<&str> {
    let Message::User { content } = message else {
        return None;
    };
    content.iter().find_map(|part| match part {
        UserContent::Text(text)
            if text
                .text
                .starts_with("当前工程快照（参考数据；更多内容请按需 inspect）：")
                || text.text.starts_with("当前工程参考数据：") =>
        {
            Some(text.text.as_str())
        }
        _ => None,
    })
}
pub fn offload_old_images(session: &mut Session, host: &impl Host) -> Result<usize, String> {
    let mut total = 0;
    for index in 0..session.messages.len().saturating_sub(2) {
        let mut replacement = session.messages[index].clone();
        let mut changed = vec![];
        let mut offloads = vec![];
        match &mut replacement {
            Message::User { content } => {
                for (image_index, part) in content.iter_mut().enumerate() {
                    if let UserContent::Image(image) = part {
                        let id = format!("image-{}", mstudio::media::id());
                        offloads.push(json!({"id":id,"image":image}));
                        *part = UserContent::text(format!(
                            "[此前图像附件 #{image_index} 已卸载；可调用 mstudio_reopen_image(imageId=\"{id}\") 重新查看。]"
                        ));
                        changed.push(image_index);
                    } else if let UserContent::ToolResult(result) = part {
                        for (nested, item) in result.content.iter_mut().enumerate() {
                            if let ToolResultContent::Image(image) = item {
                                let id = format!("image-{}", mstudio::media::id());
                                offloads.push(json!({"id":id,"image":image}));
                                *item = ToolResultContent::text(format!(
                                    "[此前工具图像 #{image_index}.{nested} 已卸载；可调用 mstudio_reopen_image(imageId=\"{id}\") 重新查看。]"
                                ));
                                changed.push(image_index);
                            }
                        }
                    }
                }
            }
            Message::Assistant { content, .. } => {
                for (image_index, part) in content.iter_mut().enumerate() {
                    if let AssistantContent::Image(image) = part {
                        let id = format!("image-{}", mstudio::media::id());
                        offloads.push(json!({"id":id,"image":image}));
                        *part = AssistantContent::text(format!(
                            "[此前助手图像 #{image_index} 已卸载；可调用 mstudio_reopen_image(imageId=\"{id}\") 重新查看。]"
                        ));
                        changed.push(image_index);
                    }
                }
            }
            Message::System { .. } => {}
        }
        if changed.is_empty() {
            continue;
        }
        host.record(
            "image/offload",
            json!({"messageIndex":index,"imageIndexes":changed,"message":replacement,"offloads":offloads}),
        )?;
        session.messages[index] = replacement;
        // The anchor survives rewrites; the meter accounts for signed reductions.
        total += changed.len();
    }
    Ok(total)
}
pub fn restore(
    store: &Store,
    project: &str,
    turn: &str,
    binding: &Value,
) -> Result<Option<Vec<Message>>, String> {
    restore_through(store, project, turn, binding, i64::MAX)
}
pub(super) fn restore_through(
    store: &Store,
    project: &str,
    turn: &str,
    binding: &Value,
    through: i64,
) -> Result<Option<Vec<Message>>, String> {
    let events = {
        let db = store.db.lock().unwrap();
        let reset: bool = db.query_row("SELECT EXISTS(SELECT 1 FROM agent_events WHERE project_id=?1 AND kind='session/reset' AND seq > COALESCE((SELECT MIN(seq) FROM agent_events WHERE project_id=?1 AND turn_id=?2),9223372036854775807))", rusqlite::params![project,turn], |r|r.get(0)).map_err(|e|e.to_string())?;
        if reset {
            return Err("原会话已重置，请重新发送消息".into());
        }
        let mut stmt = db.prepare("SELECT kind,payload FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND seq<=?3 AND kind IN ('session/start','session/message','session/compaction','image/offload','tool/result') ORDER BY seq").map_err(|e|e.to_string())?;
        stmt.query_map(rusqlite::params![project, turn, through], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };
    let mut messages = None;
    for (kind, payload) in events {
        let value: Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        if kind == "session/start" {
            if !super::binding::compatible(&value["binding"], binding) {
                return Err("原任务的模型或 Agent 已变化，请重新发送消息".into());
            }
            messages = Some(
                serde_json::from_value::<Vec<Message>>(value["messages"].clone())
                    .map_err(|e| e.to_string())?,
            );
        } else if kind == "image/offload" {
            if let Some(messages) = &mut messages {
                let index = value["messageIndex"].as_u64().ok_or("图片卸载位置无效")? as usize;
                let slot = messages.get_mut(index).ok_or("图片卸载记录已损坏")?;
                *slot =
                    serde_json::from_value(value["message"].clone()).map_err(|e| e.to_string())?;
            }
        } else if kind == "session/compaction" {
            if let Some(messages) = &mut messages {
                let start = value["start"].as_u64().ok_or("压缩起点无效")? as usize;
                let end = value["end"].as_u64().ok_or("压缩终点无效")? as usize;
                if start >= end || end > messages.len() {
                    return Err("会话压缩记录已损坏".into());
                }
                let message =
                    serde_json::from_value(value["message"].clone()).map_err(|e| e.to_string())?;
                messages.splice(start..end, [message]);
            }
        } else if let Some(messages) = &mut messages
            && let Some(message) = value.get("message")
        {
            messages.push(serde_json::from_value(message.clone()).map_err(|e| e.to_string())?);
        }
    }
    if let Some(messages) = &mut messages {
        repair_pending(messages);
    }
    Ok(messages)
}
// Never replay a side effect after a crash. Pair orphaned calls with an explicit unknown
// outcome, so the next model request can inspect the project instead of repeating a write.
pub fn repair_pending(messages: &mut Vec<Message>) {
    let mut pending = Vec::new();
    for message in messages.iter() {
        match message {
            Message::Assistant { content, .. } => {
                pending.extend(content.iter().filter_map(|c| match c {
                    AssistantContent::ToolCall(c) => Some(c.clone()),
                    _ => None,
                }))
            }
            Message::User { content } => {
                for part in content {
                    if let UserContent::ToolResult(result) = part {
                        pending.retain(|c| c.id != result.call);
                    }
                }
            }
            _ => {}
        }
    }
    for call in pending {
        messages.push(result_message(&call, &json!({"error":"上轮中断，执行结果未知；先核查当前状态，不要直接重复修改", "code":"EFFECT_UNKNOWN"})));
    }
}
pub fn partial(store: &Store, project: &str, turn: &str) -> String {
    store.db.lock().unwrap().query_row("SELECT content FROM agent_messages WHERE project_id=?1 AND role='assistant' AND json_extract(attribution,'$.turnId')=?2", rusqlite::params![project,turn], |r|r.get(0)).unwrap_or_default()
}
pub fn start(
    store: &Store,
    project: &str,
    turn: &str,
    binding: Value,
    messages: &[Message],
) -> Result<(), String> {
    journal::append(
        store,
        project,
        turn,
        "session/start",
        json!({"binding":binding,"messages":messages}),
    )
    .map_err(|e| e.to_string())
}

pub fn validate_resume(
    store: &Store,
    project: &str,
    turn: &str,
    request: &Value,
) -> Result<(), String> {
    let db = store.db.lock().unwrap();
    let (status, original):(String,String)=db.query_row("SELECT json_extract(a.attribution,'$.status'),json_extract(u.attribution,'$.request') FROM agent_messages a JOIN agent_messages u ON a.project_id=u.project_id AND json_extract(a.attribution,'$.turnId')=json_extract(u.attribution,'$.turnId') WHERE a.project_id=?1 AND json_extract(a.attribution,'$.turnId')=?2 AND a.role='assistant' AND u.role='user'",rusqlite::params![project,turn],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|_|"找不到可恢复的原任务")?;
    if !["failed", "cancelled", "interrupted"].contains(&status.as_str()) {
        return Err("原任务尚未中断，不能重复恢复".into());
    }
    let original: Value = serde_json::from_str(&original).map_err(|e| e.to_string())?;
    if &original != request {
        return Err("恢复请求与原任务不一致，请重新发送消息".into());
    }
    Ok(())
}

pub use super::session_selection::latest;
