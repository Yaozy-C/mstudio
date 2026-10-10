//! Permission filtering precedes discovery. Loading changes exposure, never authorization.
use rig_core::completion::ToolDefinition;
use serde_json::{Value, json};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};

pub const LOAD: &str = "mstudio_load_tools";
// Directly exposed from the first request. Membership grants nothing: a name here still needs
// the role's own permission, so agent-delegate only reaches roles that may delegate.
pub(crate) const BASE: &[&str] = &[
    "mstudio_read_project",
    "mstudio_read_shots",
    "mstudio_read_screenplay",
    "mstudio_read_assets",
    "mstudio_read_generation",
    "mstudio_skills",
    "mstudio_read_skill",
    "mstudio_read_image",
    "mstudio_read_result",
    // Delegation is what a coordinating role does first; making it load first cost a rejected
    // call whenever the goal was already known and the role instruction said to delegate.
    "mstudio_delegate",
];
#[derive(Clone, Default)]
pub struct LoadedTools(Arc<Mutex<HashSet<String>>>);
impl LoadedTools {
    pub fn definitions(&self, available: Vec<ToolDefinition>) -> Vec<ToolDefinition> {
        if available.is_empty() {
            return available;
        }
        let loaded = self.0.lock().unwrap();
        let deferred: Vec<_> = available
            .iter()
            .filter(|t| !BASE.contains(&t.name.as_str()))
            .collect();
        let names: Vec<_> = deferred.iter().map(|t| t.name.clone()).collect();
        let catalog = deferred
            .iter()
            .map(|t| {
                format!(
                    "{}: {}",
                    t.name,
                    t.description
                        .split(". ")
                        .next()
                        .unwrap_or("")
                        .chars()
                        .take(100)
                        .collect::<String>()
                )
            })
            .collect::<Vec<_>>()
            .join("\n");
        // The deferred catalog plus every currently exposed tool. Deferred names must stay legal or
        // they could never be loaded; exposed names must be legal too, because a model batching a
        // read tool it is about to use alongside tools it still needs would otherwise lose the
        // whole batch to validation, before load() can report those as already available.
        let mut accepted = names.clone();
        accepted.extend(
            available
                .iter()
                .filter(|t| {
                    t.name != LOAD && (BASE.contains(&t.name.as_str()) || loaded.contains(&t.name))
                })
                .map(|t| t.name.clone()),
        );
        let discovery = ToolDefinition {
            name: LOAD.into(),
            description: format!(
                "Load tools needed for the current task from this role's permitted catalog. Only currently exposed tools can be called. Load a small related batch, then use the tools on the next step. Loading executes no project action and grants no new permission. Earlier conversation calls do not imply a tool is loaded in this run. Names already exposed are accepted and reported as alreadyAvailable. Catalog:\n{catalog}"
            ),
            parameters: json!({"type":"object","properties":{"names":{"type":"array","items":{"type":"string","enum":accepted},"minItems":1,"maxItems":12}},"required":["names"],"additionalProperties":false}),
        };
        let mut active: Vec<_> = available
            .into_iter()
            .filter(|t| BASE.contains(&t.name.as_str()) || loaded.contains(&t.name))
            .collect();
        if !names.is_empty() {
            active.push(discovery);
        }
        active
    }
    // Names this role may load but that are not exposed in the current run.
    // Base tools are always exposed, so they are never listed here.
    pub fn deferred(&self, available: &[ToolDefinition]) -> Vec<String> {
        let loaded = self.0.lock().unwrap();
        available
            .iter()
            .filter(|t| !BASE.contains(&t.name.as_str()) && !loaded.contains(&t.name))
            .map(|t| t.name.clone())
            .collect()
    }
    pub fn load(&self, available: &[ToolDefinition], args: &Value) -> Value {
        let Some(names) = args["names"].as_array() else {
            return json!({"error":"Provide tool names from the catalog","code":"INVALID_ARGS"});
        };
        let permitted = |name: &str| available.iter().any(|t| t.name == name);
        // Base tools are always exposed, so naming one is harmless and must not fail the batch:
        // set them aside before the unknown-name check, which would reject the whole load step.
        let (base, deferred): (Vec<_>, Vec<_>) = names
            .iter()
            .filter_map(Value::as_str)
            .partition(|name| BASE.contains(name));
        if deferred.is_empty() {
            if base.is_empty() {
                return json!({"error":"Tool unavailable to this role; choose names from the catalog","code":"FORBIDDEN"});
            }
            return json!({
                "loaded": Vec::<String>::new(),
                "alreadyAvailable": base,
                "next": "Those tools are always available and need no loading. Nothing changed; call them directly."
            });
        }
        if names.len() > 12 || deferred.iter().any(|name| !permitted(name)) {
            return json!({"error":"Tool unavailable to this role; choose names from the catalog","code":"FORBIDDEN"});
        }
        let mut loaded = self.0.lock().unwrap();
        for name in &deferred {
            loaded.insert((*name).to_owned());
        }
        json!({
            "loaded": deferred,
            "alreadyAvailable": base,
            "next": "These tools are now available. Continue the task using them; loading has not performed the requested action."
        })
    }
}

#[cfg(test)]
#[path = "tool_loading_tests.rs"]
mod tests;
