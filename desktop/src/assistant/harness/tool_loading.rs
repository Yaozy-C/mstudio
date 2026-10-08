//! Permission filtering precedes discovery. Loading changes exposure, never authorization.
use rig_core::completion::ToolDefinition;
use serde_json::{Value, json};
use std::{collections::HashSet, sync::Mutex};

pub const LOAD: &str = "mstudio_load_tools";
const BASE: &[&str] = &[
    "mstudio_inspect",
    "mstudio_skills",
    "mstudio_read_skill",
    "mstudio_read_image",
    "mstudio_read_result",
];
#[derive(Default)]
pub struct LoadedTools(Mutex<HashSet<String>>);
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
        let discovery = ToolDefinition {
            name: LOAD.into(),
            description: format!(
                "Load tools needed for the current task from this role's permitted catalog. Only currently exposed tools can be called. Load a small related batch, then use the tools on the next step. Loading executes no project action and grants no new permission. Earlier conversation calls do not imply a tool is loaded in this run. Catalog:\n{catalog}"
            ),
            parameters: json!({"type":"object","properties":{"names":{"type":"array","items":{"type":"string","enum":names},"minItems":1,"maxItems":12}},"required":["names"],"additionalProperties":false}),
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
    pub fn load(&self, available: &[ToolDefinition], args: &Value) -> Value {
        let Some(names) = args["names"].as_array() else {
            return json!({"error":"Provide tool names from the catalog","code":"INVALID_ARGS"});
        };
        if names.is_empty()
            || names.len() > 12
            || names.iter().any(|name| {
                name.as_str().is_none_or(|name| {
                    !available
                        .iter()
                        .any(|t| t.name == name && !BASE.contains(&name))
                })
            })
        {
            return json!({"error":"Tool unavailable to this role; choose names from the catalog","code":"FORBIDDEN"});
        }
        let mut loaded = self.0.lock().unwrap();
        for name in names {
            loaded.insert(name.as_str().unwrap().into());
        }
        json!({"loaded":names,"next":"These tools are now available. Continue the task using them; loading has not performed the requested action."})
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile(id: &str) -> crate::assistant::profiles::AgentProfile {
        crate::assistant::profiles::builtins()
            .into_iter()
            .find(|p| p.id == id)
            .unwrap()
    }
    // The shipped roster has no dedicated asset role; the asset-only catalog is kept with
    // an inline profile so the atomic role-scoped loading boundary stays covered.
    fn asset_profile() -> crate::assistant::profiles::AgentProfile {
        let mut p = profile("production");
        p.tool_ids = vec![
            "project-read".into(),
            "project-assets".into(),
            "media-generation".into(),
        ];
        p
    }
    fn catalog(profile: &crate::assistant::profiles::AgentProfile) -> Vec<ToolDefinition> {
        super::super::operation_tools::tools(profile)
            .into_iter()
            .map(|t| t.definition)
            .collect()
    }
    #[test]
    fn discovery_schema_is_supported_by_the_real_scheduler() {
        for profile in crate::assistant::profiles::builtins() {
            let all = catalog(&profile);
            let scope = LoadedTools::default();
            let definitions = scope.definitions(all.clone());
            for definition in &definitions {
                super::super::schema_definition::check(&definition.parameters).unwrap();
            }
            if let Some(tool) = all.first() {
                let call = rig_core::message::ToolCall::from_wire(
                    "load",
                    rig_core::message::ToolFunction {
                        name: LOAD.into(),
                        arguments: json!({"names":[tool.name]}),
                    },
                );
                assert!(super::super::scheduler::validate(&definitions, &call).is_none());
            }
        }
    }
    #[test]
    fn main_agent_exposes_catalog_without_eager_edit_schemas() {
        let all = catalog(&profile("coordinator"));
        let scope = LoadedTools::default();
        let before = scope.definitions(all.clone());
        assert_eq!(before.len(), 1);
        assert_eq!(before[0].name, LOAD);
        let original = serde_json::to_string(&all).unwrap().len();
        assert!(serde_json::to_string(&before).unwrap().len() < original / 2);
        let names = json!({"names":["mstudio_generate_video", "mstudio_update_generation"]});
        assert!(scope.load(&all, &names).get("error").is_none());
        let active = scope.definitions(all.clone());
        assert_eq!(active.len(), 3);
        assert!(!active.iter().any(|t| t.name == "mstudio_add_caption"));
        scope.load(&all, &names);
        assert_eq!(scope.definitions(all).len(), 3);
    }
    #[test]
    fn loading_is_atomic_role_scoped_and_per_run() {
        let scope = LoadedTools::default();
        let all = catalog(&asset_profile());
        assert_eq!(
            scope.load(
                &all,
                &json!({"names":["mstudio_generate_reference_image", "mstudio_generate_video"]})
            )["code"],
            "FORBIDDEN"
        );
        assert_eq!(scope.definitions(all.clone()).len(), 1);
        scope.load(&all, &json!({"names":["mstudio_generate_reference_image"]}));
        assert_eq!(scope.definitions(all.clone()).len(), 2);
        assert_eq!(LoadedTools::default().definitions(all).len(), 1);
        assert_eq!(scope.definitions(catalog(&profile("concept"))).len(), 1);
    }
    #[test]
    fn scheduler_rejects_unloaded_tools_then_accepts_loaded_parameters() {
        let scope = LoadedTools::default();
        let all = catalog(&profile("production"));
        let call = rig_core::message::ToolCall::from_wire(
            "c",
            rig_core::message::ToolFunction {
                name: "mstudio_set_video_prompt".into(),
                arguments: json!({"id":"shot", "prompt":"Move"}),
            },
        );
        assert!(
            super::super::scheduler::validate(&scope.definitions(all.clone()), &call).is_some()
        );
        scope.load(&all, &json!({"names":["mstudio_set_video_prompt"]}));
        assert!(super::super::scheduler::validate(&scope.definitions(all), &call).is_none());
    }
}
