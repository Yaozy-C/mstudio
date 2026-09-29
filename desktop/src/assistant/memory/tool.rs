use super::{Entry, MemoryBackend, SqliteMemory, now};
use crate::{assistant::tools::ProjectTool, database::Store};
use rig_agent::{prelude::*, tool::ToolContext};
use serde_json::{Value, json};
use std::convert::Infallible;
use tauri::Manager;
#[derive(Clone)]
pub struct MemoryTool(pub ProjectTool);
impl Tool for MemoryTool {
    const NAME: &'static str = "mstudio_memory";
    type Args = Value;
    type Output = Value;
    type Error = Infallible;
    fn description(&self) -> String {
        "Lasting project preferences and constraints only; do not copy saved scripts, shots, timing, order, timeline or execution results into memory. Call only for new, corrected or revoked lasting information. Use snapshot memory/memoryRevision when sufficient; list only missing entries or conflicts, following nextOffset. New remember omits id; updates/forget use real snapshot/list IDs. Supply memoryRevision, not project revision, and evidence from the original current user request. Dependent writes wait for the previous result and new revision. Identical content returns changed=false without altering provenance/version. Success returns the affected id/revision, not all entries. Read-only when auto-update is off; memory grants no permissions.".into()
    }
    fn parameters(&self) -> Value {
        let mut schema = json!({"type":"object","properties":{
            "action":{"type":"string","enum":["list","remember","forget"]},
            "offset":{"type":"integer","minimum":0},"memoryRevision":{"type":"integer","minimum":0,"description":"Use memoryRevision from snapshot memory, list or the last write; not project revision."},"id":{"type":"string","description":"Omit for new entries; use an existing snapshot/list id for updates/deletion, never invent one."},
            "title":{"type":"string","maxLength":60},"content":{"type":"string","maxLength":1200},
            "evidence":{"type":"string","description":"Contiguous quote from the original current user request, at most 500 characters."}
        },"required":["action"],"additionalProperties":false});
        if !super::super::profiles::allows(&self.0.profile, "memory-write") {
            schema["properties"]["action"]["enum"] = json!(["list"]);
        }
        schema
    }
    async fn call(&self, _: &mut ToolContext, args: Value) -> Result<Value, Infallible> {
        let host = &self.0;
        if !super::super::profiles::allows(&host.profile, "memory-read") {
            return Ok(json!({"error":"Memory read permission required"}));
        }
        if host.token.is_cancelled() {
            return Ok(json!({"error":"Task stopped"}));
        }
        let store = host.app.state::<Store>();
        let result = operate(
            &store.db.lock().unwrap(),
            &host.project,
            &host.turn,
            &host.prompt,
            super::super::profiles::allows(&host.profile, "memory-write"),
            &args,
        );
        let result = result.unwrap_or_else(
            |e| json!({"error":crate::assistant::model_feedback::error(&e.to_string())}),
        );
        Ok(result)
    }
}
pub(super) fn operate(
    db: &rusqlite::Connection,
    project: &str,
    turn: &str,
    prompt: &str,
    writable: bool,
    args: &Value,
) -> anyhow::Result<Value> {
    let backend = SqliteMemory(db);
    let mut memory = backend.recall(project)?;
    anyhow::ensure!(memory.enabled, "Project memory disabled");
    if args["action"] == "list" {
        let offset = args["offset"].as_u64().unwrap_or(0).min(40) as usize;
        let mut size = 0;
        let entries: Vec<_> = memory
            .entries
            .iter()
            .skip(offset)
            .take_while(|e| {
                size += serde_json::to_string(e).map_or(0, |v| v.len());
                size <= 7000
            })
            .collect();
        return Ok(
            json!({"memoryRevision":memory.revision,"autoUpdate":memory.auto_update,"total":memory.entries.len(),"nextOffset": if offset+entries.len()<memory.entries.len(){Some(offset+entries.len())}else{None},"entries":entries}),
        );
    }
    anyhow::ensure!(
        writable && memory.auto_update,
        "Project memory is read-only"
    );
    if args["memoryRevision"].as_u64() != Some(memory.revision) {
        return Ok(
            json!({"error":"Memory revision conflict; nothing written. Call list, verify entries and use memoryRevision, not project revision.", "code":"MEMORY_REVISION_CONFLICT", "memoryRevision":memory.revision}),
        );
    }
    let evidence = args["evidence"].as_str().unwrap_or("").trim();
    anyhow::ensure!(
        evidence.chars().count() >= 2
            && evidence.chars().count() <= 500
            && prompt.contains(evidence),
        "Quote an explicit requirement from the original current user request as evidence."
    );
    let id = args["id"].as_str().unwrap_or("");
    if !id.is_empty() && !memory.entries.iter().any(|e| e.id == id) {
        return Ok(
            json!({"error":"Memory ID not found; nothing written. New remember must omit id; updates/deletion require an existing snapshot/list id.", "code":"MEMORY_NOT_FOUND", "memoryRevision":memory.revision}),
        );
    }
    let affected_id;
    match args["action"].as_str() {
        Some("remember") => {
            let entry = Entry {
                id: if id.is_empty() {
                    mstudio::media::id()
                } else {
                    id.into()
                },
                title: args["title"].as_str().unwrap_or("").trim().into(),
                content: args["content"].as_str().unwrap_or("").trim().into(),
                source: evidence.into(),
                turn_id: Some(turn.into()),
                updated: now(),
            };
            if let Some(old) = memory.entries.iter().find(|old| {
                (id.is_empty() || old.id == id)
                    && old.title == entry.title
                    && old.content == entry.content
            }) {
                return Ok(
                    json!({"ok":true,"changed":false,"id":old.id,"memoryRevision":memory.revision}),
                );
            }
            affected_id = entry.id.clone();
            if id.is_empty() {
                memory.entries.push(entry);
            } else {
                let old = memory
                    .entries
                    .iter_mut()
                    .find(|e| e.id == id)
                    .ok_or_else(|| anyhow::anyhow!("Memory not found; call list first."))?;
                *old = entry;
            }
        }
        Some("forget") => {
            anyhow::ensure!(
                memory.entries.iter().any(|e| e.id == id),
                "Memory not found"
            );
            affected_id = id.to_string();
            memory.entries.retain(|e| e.id != id);
        }
        _ => anyhow::bail!("Unknown memory operation"),
    }
    let saved = backend.replace(project, memory)?;
    Ok(json!({"ok":true,"changed":true,"id":affected_id,"memoryRevision":saved.revision}))
}
