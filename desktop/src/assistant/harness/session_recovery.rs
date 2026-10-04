//! Restore unfinished tools from authoritative domain facts; never replay writes.
use super::session::result_message;
use crate::database::Store;
use rig_core::message::{AssistantContent, Message, ToolCall, UserContent};
use serde_json::{Value, json};

pub fn repair(store: &Store, project: &str, turn: &str, messages: &mut Vec<Message>) {
    repair_pending(messages, |call| {
        if call.function.name == "mstudio_await_generation" {
            let keys = call.function.arguments["taskKeys"]
                .as_array()?
                .iter()
                .filter_map(|key| key.as_str().map(str::to_owned))
                .collect::<Vec<_>>();
            let mut value = crate::project_service::generation_wait::snapshot(
                &store.db.lock().unwrap(),
                project,
                &keys,
            )
            .ok()?;
            value["waitEnded"] = json!("interrupted");
            return Some(value);
        }
        if !super::operation_tools::is_edit(&call.function.name) {
            return None;
        }
        crate::project_service::receipts::read(
            &store.db.lock().unwrap(),
            project,
            &format!("{turn}:{}", call.id.as_str()),
        )
        .ok()
        .flatten()
    });
}

// Never replay a side effect after a crash. Pair orphaned calls with an explicit unknown
// outcome, so the next model request can inspect the project instead of repeating a write.
pub fn repair_pending(
    messages: &mut Vec<Message>,
    mut receipt: impl FnMut(&ToolCall) -> Option<Value>,
) {
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
        let value = receipt(&call).unwrap_or_else(|| json!({"error":"Previous turn interrupted; effects unknown. Inspect current state before repeating edits.", "code":"EFFECT_UNKNOWN","stage":"recovery","outcome":"unknown"}));
        messages.push(result_message(&call, &value));
    }
}
