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
    // Mirrors available_definitions(): the role's read tools, its deferred catalog and the
    // framework tools the registry adds separately. Omitting any group would let the enum look
    // correct while production exposes names this fixture never considers.
    fn available(profile: &crate::assistant::profiles::AgentProfile) -> Vec<ToolDefinition> {
        let mut all = super::super::read_tools::definitions();
        all.extend(catalog(profile));
        let frame = |name: &str| ToolDefinition {
            name: name.into(),
            description: name.into(),
            parameters: json!({"type":"object","properties":{},"additionalProperties":false}),
        };
        for (tool, allowed) in [
            (
                "mstudio_models",
                crate::assistant::profiles::allows(profile, "models"),
            ),
            (
                "mstudio_await_generation",
                crate::assistant::profiles::allows(profile, "models"),
            ),
            ("mstudio_read_result", true),
        ] {
            if allowed {
                all.push(frame(tool));
            }
        }
        all
    }
    fn discovery_enum(definitions: &[ToolDefinition]) -> Vec<String> {
        definitions
            .iter()
            .find(|d| d.name == LOAD)
            .unwrap()
            .parameters["properties"]["names"]["items"]["enum"]
            .as_array()
            .unwrap()
            .iter()
            .filter_map(Value::as_str)
            .map(str::to_owned)
            .collect()
    }
    #[test]
    fn discovery_enum_lists_every_name_the_role_may_select() {
        for profile in crate::assistant::profiles::builtins() {
            let all = available(&profile);
            let scope = LoadedTools::default();
            let definitions = scope.definitions(all.clone());
            for definition in &definitions {
                super::super::schema_definition::check(&definition.parameters).unwrap();
            }
            let selectable = discovery_enum(&definitions);
            // Every permitted tool is either loadable now or already exposed, so the enum is the
            // whole callable set: shrinking it to loaded tools alone would block all loading.
            for tool in &all {
                assert!(
                    selectable.contains(&tool.name),
                    "{} missing in {}",
                    tool.name,
                    profile.id
                );
            }
            // Neither the loader itself nor a name outside the role may be selected.
            assert!(!selectable.contains(&LOAD.to_owned()));
            assert!(
                super::super::scheduler::validate(
                    &definitions,
                    &rig_core::message::ToolCall::from_wire(
                        "load",
                        rig_core::message::ToolFunction {
                            name: LOAD.into(),
                            arguments: json!({"names":["mstudio_load_image"]}),
                        },
                    ),
                    &[]
                )
                .is_some()
            );
            let first = all[0].name.clone();
            scope.load(&all, &json!({"names":[first.clone()]}));
            assert!(discovery_enum(&scope.definitions(all)).contains(&first));
        }
    }
    #[test]
    fn a_batch_mixing_an_exposed_tool_with_needed_tools_is_not_rejected() {
        // The real failure this guards: a subagent asked for five tools it needed plus
        // mstudio_read_generation, which it already had. Validation rejected the whole batch, so
        // none of the five were loaded and the run needed another step.
        let mut image = profile("image");
        image.tool_ids.push("media-generation".into());
        let scope = LoadedTools::default();
        let all = available(&image);
        let definitions = scope.definitions(all.clone());
        let requested = json!({"names":[
            "mstudio_generate_image", "mstudio_set_image_prompt", "mstudio_set_shot_frames",
            "mstudio_models", "mstudio_await_generation", "mstudio_read_generation"
        ]});
        let call = rig_core::message::ToolCall::from_wire(
            "load",
            rig_core::message::ToolFunction {
                name: LOAD.into(),
                arguments: requested.clone(),
            },
        );
        // The rejected name must be selectable, exactly as production exposes it.
        let selectable = discovery_enum(&definitions);
        for name in requested["names"].as_array().unwrap() {
            let name = name.as_str().unwrap();
            assert!(
                selectable.contains(&name.to_owned()),
                "{name} must be selectable"
            );
        }
        assert!(super::super::scheduler::validate(&definitions, &call, &[]).is_none());
        let value = scope.load(&all, &requested);
        assert_eq!(
            value["alreadyAvailable"],
            json!(["mstudio_read_generation"])
        );
        assert_eq!(value["loaded"].as_array().unwrap().len(), 5);
        // A name that is neither exposed nor permitted still fails.
        let bogus = rig_core::message::ToolCall::from_wire(
            "load",
            rig_core::message::ToolFunction {
                name: LOAD.into(),
                arguments: json!({"names":["mstudio_load_image"]}),
            },
        );
        assert!(super::super::scheduler::validate(&definitions, &bogus, &[]).is_some());
    }
    #[test]
    fn video_agent_exposes_catalog_without_eager_edit_schemas() {
        let all = catalog(&profile("production"));
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
        assert_eq!(scope.definitions(catalog(&profile("writer"))).len(), 1);
        assert!(scope.definitions(catalog(&profile("concept"))).is_empty());
    }
    #[test]
    fn deferred_names_drive_a_recoverable_rejection_then_disappear_once_loaded() {
        let scope = LoadedTools::default();
        let all = catalog(&profile("production"));
        let deferred = scope.deferred(&all);
        assert!(
            deferred
                .iter()
                .any(|name| name == "mstudio_set_video_prompt")
        );
        assert!(!deferred.iter().any(|name| name == LOAD));
        for base in BASE {
            assert!(!deferred.iter().any(|name| name == base), "{base}");
        }
        let call = rig_core::message::ToolCall::from_wire(
            "c",
            rig_core::message::ToolFunction {
                name: "mstudio_set_video_prompt".into(),
                arguments: json!({"id":"shot", "prompt":"Move"}),
            },
        );
        let rejected =
            super::super::scheduler::validate(&scope.definitions(all.clone()), &call, &deferred)
                .unwrap();
        assert_eq!(rejected["code"], "UNKNOWN_TOOL");
        assert!(
            rejected["error"]
                .as_str()
                .unwrap()
                .contains("mstudio_load_tools")
        );
        scope.load(&all, &json!({"names":["mstudio_set_video_prompt"]}));
        assert!(
            !scope
                .deferred(&all)
                .iter()
                .any(|n| n == "mstudio_set_video_prompt")
        );
    }
    #[test]
    fn naming_an_always_available_tool_reports_it_instead_of_failing_the_batch() {
        // The operation catalog holds no base tools, matching what load() receives in production
        // for the name a model is most likely to name: base tools are judged before FORBIDDEN.
        let scope = LoadedTools::default();
        let all = catalog(&profile("production"));
        assert!(!all.iter().any(|t| t.name == "mstudio_read_project"));
        let value = scope.load(&all, &json!({"names":["mstudio_read_project"]}));
        assert_eq!(value["alreadyAvailable"], json!(["mstudio_read_project"]));
        assert!(value.get("error").is_none());
        // An unknown or unpermitted name still fails, so loading grants no permission.
        assert_eq!(
            scope.load(&all, &json!({"names":["mstudio_not_a_tool"]}))["code"],
            "FORBIDDEN"
        );
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
            super::super::scheduler::validate(&scope.definitions(all.clone()), &call, &[])
                .is_some()
        );
        scope.load(&all, &json!({"names":["mstudio_set_video_prompt"]}));
        assert!(super::super::scheduler::validate(&scope.definitions(all), &call, &[]).is_none());
    }
}
