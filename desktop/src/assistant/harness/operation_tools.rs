//! Model-facing task tools; the domain still receives transactional operations.
use crate::assistant::profiles::AgentProfile;
use rig_core::completion::ToolDefinition;
use serde_json::{Value, json};

pub struct OperationTool {
    pub definition: ToolDefinition,
    op: String,
    fixed: serde_json::Map<String, Value>,
    nested: Option<String>,
    task_prompt: bool,
    batch: bool,
}
impl OperationTool {
    pub fn arguments(&self, value: Value) -> Value {
        let values = if self.batch {
            value["items"].as_array().unwrap().clone()
        } else {
            vec![value]
        };
        let operations: Vec<_> = values
            .into_iter()
            .map(|mut value| {
                let fields = value.as_object_mut().unwrap();
                if self.task_prompt
                    && let Some(prompt) = fields.remove("prompt")
                {
                    fields.insert("text".into(), prompt);
                }
                if let Some(nested) = &self.nested {
                    let mut inner = fields.clone();
                    for key in [
                        "id",
                        "title",
                        "text",
                        "assetId",
                        "resultAssetId",
                        "references",
                        "x",
                        "y",
                    ] {
                        inner.remove(key);
                    }
                    fields.retain(|key, _| {
                        [
                            "id",
                            "title",
                            "text",
                            "assetId",
                            "resultAssetId",
                            "references",
                            "x",
                            "y",
                        ]
                        .contains(&key.as_str())
                    });
                    fields.insert(nested.clone(), json!(inner));
                }
                fields.extend(self.fixed.clone());
                fields.insert("op".into(), json!(self.op));
                value
            })
            .collect();
        json!({"action":"edit","operations":operations})
    }
}
fn add(
    out: &mut Vec<OperationTool>,
    name: &str,
    op: &str,
    mut schema: Value,
    fixed: serde_json::Map<String, Value>,
    nested: Option<&str>,
    description: &str,
) {
    let task_prompt = matches!(op, "request_generation" | "update_generation");
    let properties = schema["properties"].as_object_mut().unwrap();
    properties.remove("op");
    for key in fixed.keys() {
        properties.remove(key);
    }
    if task_prompt && let Some(mut text) = properties.remove("text") {
        text["description"] = json!(
            "Complete final prompt for this generation task. Passed unchanged as the task prompt; not automatically appended to the shot or script."
        );
        properties.insert("prompt".into(), text);
    }
    if nested.is_some() {
        for (key, description) in [
            (
                "prompt",
                "Complete video prompt draft. Saving does not generate media.",
            ),
            (
                "framePrompt",
                "Complete still-image prompt draft. Saving does not generate media.",
            ),
            (
                "duration",
                "Editorial duration in seconds; independent of generated source clip duration.",
            ),
            (
                "scriptId",
                "Existing paragraph ID inside the selected screenplayId.",
            ),
        ] {
            if let Some(field) = properties.get_mut(key) {
                field["description"] = json!(description);
            }
        }
    }
    if op == "request_generation"
        && let Some(id) = properties.get_mut("id")
    {
        id["description"] = json!(
            "Existing target shot ID. Omission inherits the referenced task's shot when present; otherwise creates a standalone task. Use mstudio_generate_reference_image for reusable reference assets."
        );
    }
    let keys: Vec<_> = properties.keys().cloned().collect();
    if let Some(required) = schema["required"].as_array_mut() {
        for key in required.iter_mut() {
            if task_prompt && key == "text" {
                *key = json!("prompt");
            }
        }
        required.retain(|key| {
            key.as_str()
                .is_some_and(|key| keys.iter().any(|k| k == key))
        });
    }
    // Examples inherited from the internal operation schema use a different shape.
    schema.as_object_mut().unwrap().remove("description");
    let description = format!(
        "{description} Supply the listed parameters directly, without op, operations, patch or data wrappers. Omitted fields are preserved. A committed receipt is authoritative; read only missing details. Changes outside this tool's fields belong to another tool/role."
    );
    out.push(OperationTool {
        definition: ToolDefinition {
            name: format!("mstudio_{name}"),
            description,
            parameters: schema,
        },
        op: op.into(),
        fixed,
        nested: nested.map(str::to_owned),
        task_prompt,
        batch: false,
    });
}
fn flat(schema: &Value, nested: &str, selected: &[&str]) -> Value {
    let mut properties = serde_json::Map::new();
    properties.insert("id".into(), schema["properties"]["id"].clone());
    if let Some(fields) = schema["properties"][nested]["properties"].as_object() {
        for (name, value) in fields {
            if selected.is_empty() || selected.contains(&name.as_str()) {
                properties.insert(name.clone(), value.clone());
            }
        }
    }
    json!({"type":"object","properties":properties,"required":["id"],"additionalProperties":false})
}
mod definitions;
pub use definitions::tools;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_coordinator;

pub fn is_edit(name: &str) -> bool {
    static NAMES: std::sync::OnceLock<std::collections::HashSet<String>> =
        std::sync::OnceLock::new();
    name == "mstudio_edit"
        || NAMES
            .get_or_init(|| {
                let mut profile = crate::assistant::profiles::builtins().remove(0);
                profile.tool_ids = crate::assistant::profiles::TOOL_IDS
                    .iter()
                    .map(|id| (*id).into())
                    .collect();
                tools(&profile)
                    .into_iter()
                    .map(|tool| tool.definition.name)
                    .collect()
            })
            .contains(name)
}
/// Translate parameters already checked by the scheduler's tool boundary.
pub fn decode(profile: &AgentProfile, name: &str, args: Value) -> Option<Value> {
    tools(profile)
        .into_iter()
        .find(|tool| tool.definition.name == name)
        .map(|tool| tool.arguments(args))
}
