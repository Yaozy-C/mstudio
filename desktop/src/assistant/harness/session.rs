//! The exact provider messages are private session events, never UI tool-card payloads.
use super::Host;
#[cfg(test)]
pub use super::session_recovery::repair_pending;
use crate::{assistant::journal, database::Store};
use rig_core::message::{Message, ToolCall, ToolResultContent, UserContent};
use serde_json::{Value, json};

pub struct Session {
    pub delegation_outcomes: super::outcomes::Outcomes,
    pub messages: Vec<Message>,
    pub text: String,
    pub usage_anchor: Option<super::budget::UsageAnchor>,
    pub provider_context_window: Option<usize>,
}
impl Session {
    pub fn new(messages: Vec<Message>) -> Self {
        Self {
            delegation_outcomes: super::outcomes::Outcomes::from_messages(&messages),
            messages,
            text: String::new(),
            usage_anchor: None,
            provider_context_window: None,
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
    pub fn progress(&mut self, host: &impl Host, text: &str) -> Result<(), String> {
        if !text.trim().is_empty() {
            host.record("assistant/progress", json!({"text":text}))?;
        }
        self.replace_visible(host, "")
    }
    pub fn replace_visible(&mut self, host: &impl Host, text: &str) -> Result<(), String> {
        if self.text != text {
            self.text = text.into();
            host.record("assistant/partial", json!({"text":self.text,"delta":""}))?;
        }
        Ok(())
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
    if let Some(media) = value.get("__offloadedMedia")
        && let Ok(media) = serde_json::from_value::<UserContent>(media.clone())
        && matches!(
            media,
            UserContent::Audio(_)
                | UserContent::Video(_)
                | UserContent::Document(_)
                | UserContent::Text(_)
        )
    {
        return Message::User {
            content: vec![
                UserContent::ToolResult(call.result(vec![ToolResultContent::text(format!(
                    "Media {} reloaded.",
                    value["mediaId"]
                ))])),
                media,
            ],
        };
    }
    if let Some(image) = value.get("__offloadedImage")
        && let Ok(image) = serde_json::from_value::<rig_core::message::Image>(image.clone())
    {
        return Message::User {
            content: vec![UserContent::ToolResult(call.result(vec![
                ToolResultContent::text(format!(
                    "Image {} {} for visual analysis.",
                    value["imageId"].as_str().unwrap_or(""),
                    if call.function.name == "mstudio_read_image" {
                        "loaded"
                    } else {
                        "reopened"
                    }
                )),
                ToolResultContent::Image(image),
            ]))],
        };
    }
    let error = value.get("error").filter(|v| !v.is_null());
    let text = if error.is_some() && value.as_object().is_some_and(|v| v.len() > 1) {
        // Keep structured validation fields used by project tools while marking failure natively.
        format!("Error: {value}")
    } else if let Some(error) = error {
        format!(
            "Error: {}",
            error
                .as_str()
                .map(str::to_owned)
                .unwrap_or_else(|| error.to_string())
        )
    } else {
        value
            .as_str()
            .map(str::to_owned)
            .unwrap_or_else(|| value.to_string())
    };
    let mut result = call.result(vec![ToolResultContent::text(text)]);
    result.is_error = error.is_some();
    Message::User {
        content: vec![UserContent::ToolResult(result)],
    }
}

/// Detect a project snapshot so an unchanged snapshot is not injected twice.
pub fn project_snapshot_text(message: &Message) -> Option<&str> {
    super::context_source::source(message).map(|(text, _)| text)
}
/// Only a later assistant response proves that a batch reached the model.
/// Keep the entire newest batch, including reopened images and injected context.
pub(super) fn consumed_prefix(messages: &[Message]) -> usize {
    messages
        .iter()
        .rposition(|m| matches!(m, Message::Assistant(_)))
        .unwrap_or(0)
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
    let events = super::session_events::read(store, project, turn, through)?;
    let mut messages = None;
    for (kind, value) in events {
        if kind == "session/start" {
            if value["binding"]["archiveContext"] != binding["archiveContext"] {
                return Ok(None);
            }
            if !super::binding::compatible(&value["binding"], binding) {
                return Err("原任务的模型或 Agent 已变化，请重新发送消息".into());
            }
            messages = Some(
                serde_json::from_value::<Vec<Message>>(value["messages"].clone())
                    .map_err(|e| e.to_string())?,
            );
        } else if kind == "media/offload" {
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
        super::session_recovery::repair(store, project, turn, messages)?;
    }
    Ok(messages)
}
pub fn partial_in(db: &rusqlite::Connection, project: &str, turn: &str) -> anyhow::Result<String> {
    Ok(db.query_row("SELECT content FROM agent_messages WHERE project_id=?1 AND role='assistant' AND json_extract(attribution,'$.turnId')=?2", rusqlite::params![project,turn], |r|r.get(0))?)
}
pub fn start(
    store: &Store,
    project: &str,
    turn: &str,
    binding: Value,
    messages: &[Message],
) -> Result<(), String> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction().map_err(|error| error.to_string())?;
    start_in(&tx, project, turn, binding, messages).map_err(|error| error.to_string())?;
    tx.commit().map_err(|error| error.to_string())
}
pub fn start_in(
    db: &rusqlite::Connection,
    project: &str,
    turn: &str,
    binding: Value,
    messages: &[Message],
) -> anyhow::Result<()> {
    for message in messages {
        if let Some((_, source)) = super::context_source::source(message)
            && source["projectId"] == project
            && let Some(versions) = source["versions"].as_object()
        {
            for (key, version) in versions {
                db.execute("INSERT INTO project_observations VALUES(?1,?2,?3,?4) ON CONFLICT(project_id,turn_id,target) DO UPDATE SET value=excluded.value", rusqlite::params![project,turn,key,version.as_str()])?;
            }
        }
    }
    journal::insert(
        db,
        project,
        turn,
        "session/start",
        json!({"binding":binding,"messages":messages,"contextMode":"continuous"}),
    )
}

pub fn validate_resume(
    store: &Store,
    project: &str,
    turn: &str,
    request: &Value,
) -> Result<(), String> {
    let db = store.db.lock().unwrap();
    let (status, original):(String,String)=db.query_row("SELECT json_extract(a.attribution,'$.status'),json_extract(u.attribution,'$.request') FROM agent_messages a JOIN agent_messages u ON a.project_id=u.project_id AND json_extract(a.attribution,'$.turnId')=json_extract(u.attribution,'$.turnId') WHERE a.project_id=?1 AND json_extract(a.attribution,'$.turnId')=?2 AND a.role='assistant' AND u.role='user'",rusqlite::params![project,turn],|r|Ok((r.get(0)?,r.get(1)?))).map_err(|error|match error {rusqlite::Error::QueryReturnedNoRows=>"找不到可恢复的原任务".to_owned(),error=>error.to_string()})?;
    if !["failed", "cancelled", "interrupted"].contains(&status.as_str()) {
        return Err("原任务尚未中断，不能重复恢复".into());
    }
    let original: Value = serde_json::from_str(&original).map_err(|e| e.to_string())?;
    if &original != request {
        return Err("恢复请求与原任务不一致，请重新发送消息".into());
    }
    Ok(())
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
