//! Model-facing task tools; the domain still receives transactional operations.
use crate::assistant::{profiles::AgentProfile, tool_schema};
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
    task_prompt: bool,
    description: &str,
) {
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
pub fn tools(profile: &AgentProfile) -> Vec<OperationTool> {
    let schema = tool_schema::for_profile(profile);
    let mut out = Vec::new();
    for operation in schema["properties"]["operations"]["items"]["oneOf"]
        .as_array()
        .into_iter()
        .flatten()
    {
        let op = operation["properties"]["op"]["const"].as_str().unwrap();
        if op == "request_generation" {
            let kind = operation["properties"]["mediaKind"]["const"]
                .as_str()
                .unwrap();
            let asset = operation["properties"].get("generationPurpose").is_some();
            let name = if asset {
                "generate_reference_image"
            } else if kind == "video" {
                "generate_video"
            } else {
                "generate_image"
            };
            let mut fixed = serde_json::Map::from_iter([("mediaKind".into(), json!(kind))]);
            if asset {
                fixed.insert("generationPurpose".into(), json!("asset"));
            }
            add(
                &mut out,
                name,
                op,
                operation.clone(),
                fixed,
                None,
                true,
                "Create a generation task using prompt and actual media references. Use the selected model's capabilities for parameters. Prompt drafting alone does not authorize generation; task submission is not completion.",
            );
        } else if op == "add_node" {
            for kind in operation["properties"]["kind"]["enum"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                let mut parameters = operation.clone();
                let properties = parameters["properties"].as_object_mut().unwrap();
                properties.retain(|key, _| {
                    crate::assistant::permissions::allows_node_field(profile, kind, key)
                });
                let nested = match kind {
                    "shot" => Some("shot"),
                    "screenplay" => Some("screenplay"),
                    _ => None,
                };
                let fields = nested
                    .and_then(|name| properties.get(name))
                    .and_then(|value| value["properties"].as_object())
                    .cloned();
                properties.remove("shot");
                properties.remove("screenplay");
                if let Some(fields) = fields {
                    properties.extend(fields);
                }
                add(
                    &mut out,
                    &format!("add_{kind}"),
                    op,
                    parameters,
                    serde_json::Map::from_iter([("kind".into(), json!(kind))]),
                    nested,
                    false,
                    &format!(
                        "Create a {kind} node with a new id and title. All listed fields are direct parameters, including any script or shot fields. x/y, when supplied, are absolute canvas coordinates."
                    ),
                );
            }
        } else if op == "update_node" {
            if operation["properties"].get("shot").is_some() {
                for (name, fields, description) in [
                    (
                        "set_video_prompt",
                        vec!["prompt"],
                        "Save the video prompt draft on a shot. Does not generate video or alter dialogue, timing, staging or an existing generation task.",
                    ),
                    (
                        "set_image_prompt",
                        vec!["framePrompt"],
                        "Save the still-image prompt draft on a shot. Does not generate an image.",
                    ),
                    (
                        "set_shot_frames",
                        vec!["frames"],
                        "Set the shot's storyboard frame assets.",
                    ),
                    (
                        "update_shot",
                        vec!["screenplayId", "scriptId", "order", "duration", "dialogue"],
                        "Update shot structure, editorial timing or dialogue. This does not edit generation prompts.",
                    ),
                ] {
                    let mut parameters = flat(operation, "shot", &fields);
                    if parameters["properties"].as_object().unwrap().len() == 1 {
                        continue;
                    }
                    if fields.len() == 1 {
                        parameters["required"] = json!(["id", fields[0]]);
                        parameters["properties"][fields[0]]["description"] = json!(description);
                    }
                    add(
                        &mut out,
                        name,
                        op,
                        parameters,
                        Default::default(),
                        Some("shot"),
                        false,
                        description,
                    );
                    if name == "update_shot" {
                        let source = out.last().unwrap();
                        out.push(OperationTool { definition: ToolDefinition { name:"mstudio_update_shots".into(),description:"Atomically update multiple shots, including order swaps. Each items entry has the same direct fields as update_shot; no op or shot wrapper. The entire batch succeeds or fails together.".into(),parameters:json!({"type":"object","properties":{"items":{"type":"array","minItems":1,"maxItems":30,"items":source.definition.parameters}},"required":["items"],"additionalProperties":false}) },op:op.into(),fixed:Default::default(),nested:Some("shot".into()),task_prompt:false,batch:true });
                    }
                }
            }
            if operation["properties"].get("screenplay").is_some() {
                add(
                    &mut out,
                    "update_screenplay",
                    op,
                    flat(operation, "screenplay", &[]),
                    Default::default(),
                    Some("screenplay"),
                    false,
                    "Update screenplay paragraphs by ID. Merge preserves omitted paragraphs; replace requires complete paragraphs. Explicit removeParagraphIds deletes paragraphs.",
                );
            }
            let mut node = operation.clone();
            for field in ["shot", "screenplay", "references"] {
                node["properties"].as_object_mut().unwrap().remove(field);
            }
            if node["properties"].as_object().unwrap().len() > 2 {
                add(
                    &mut out,
                    "update_node",
                    op,
                    node,
                    Default::default(),
                    None,
                    false,
                    "Update the node's title, body or asset fields. Shot body text is staging; prompts and shot structure have dedicated tools.",
                );
            }
        } else if op == "set_references" {
            let modes = operation["properties"]["referenceMode"]["enum"]
                .as_array()
                .unwrap();
            for mode in modes {
                let mode = mode.as_str().unwrap();
                add(
                    &mut out,
                    &format!("{mode}_references"),
                    op,
                    operation.clone(),
                    serde_json::Map::from_iter([("referenceMode".into(), json!(mode))]),
                    None,
                    false,
                    match mode {
                        "upsert" => {
                            "Add or update node references by asset ID; preserve other references."
                        }
                        "replace" => {
                            "Replace the complete reference list only when a full reset is intended."
                        }
                        _ => "Remove the listed reference asset IDs from the node.",
                    },
                );
            }
        } else {
            let description = match op {
                "update_generation" => {
                    "Save a complete replacement prompt on the existing taskKey. Does not regenerate or change the shot draft."
                }
                "regenerate_generation" => {
                    "Create a new run from an existing taskKey, retaining its model, inputs and parameters. Requires an explicit regeneration request."
                }
                "add_node" => {
                    "Create a project node with its kind, ID and title. Use only fields permitted for that kind."
                }
                "remove_node" => "Delete the node identified by id.",
                "set_brief" => "Replace the project brief with text.",
                "set_creation" => {
                    "Update the project creative intent, essential requirements, preserved decisions or stage."
                }
                "choose_take" => "Choose assetId as the selected take for shot id.",
                "assemble_screenplay" => {
                    "Assemble the screenplay identified by id into the timeline using its shots."
                }
                "append_clip" => {
                    "Append assetId to the timeline with optional placement, source trim and visual settings. Times are seconds."
                }
                "update_clip" => {
                    "Update the timeline clip identified by id. Times are seconds; speed is an absolute playback multiplier."
                }
                "move_clip" => {
                    "Move a timeline clip to start (output seconds), optionally changing trackId. allowOverlap explicitly allows overlap."
                }
                "retime_clip" => {
                    "Set a clip's playback speed. ripple shifts subsequent clips when requested."
                }
                "slip_clip" => {
                    "Shift the clip source range by sourceOffset seconds without changing its output placement."
                }
                "remove_clip" => "Remove the timeline clip identified by id.",
                "set_transition" => {
                    "Set the transition from fromClipId into clip id. duration is seconds."
                }
                "add_track" => "Create a named video or audio timeline track.",
                "update_track" => "Update a track's title, muted or hidden state by id.",
                "add_caption" => {
                    "Create a caption with text and output start/end times in seconds."
                }
                "update_caption" => {
                    "Update caption text or output start/end times in seconds by id."
                }
                "remove_caption" => "Delete the caption identified by id.",
                _ => "Apply this named project operation within the current role's permissions.",
            };
            add(
                &mut out,
                op,
                op,
                operation.clone(),
                Default::default(),
                None,
                op == "update_generation",
                description,
            );
        }
    }
    out
}
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
pub fn decode(profile: &AgentProfile, name: &str, args: Value) -> Option<Result<Value, Value>> {
    tools(profile)
        .into_iter()
        .find(|tool| tool.definition.name == name)
        .map(|tool| {
            let issues = super::schema::issues(&tool.definition.parameters, &args);
            if issues.is_empty() {
                Ok(tool.arguments(args))
            } else {
                Err(super::schema::rejection(issues))
            }
        })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn profile(id: &str) -> AgentProfile {
        crate::assistant::profiles::builtins()
            .into_iter()
            .find(|p| p.id == id)
            .unwrap()
    }
    #[test]
    fn production_has_direct_prompt_tools_without_storyboard_writes() {
        let p = profile("production");
        let available = tools(&p);
        for name in [
            "mstudio_edit",
            "mstudio_update_shot",
            "mstudio_update_shots",
            "mstudio_update_node",
            "mstudio_update_screenplay",
        ] {
            assert!(
                !available.iter().any(|t| t.definition.name == name),
                "{name}"
            );
        }
        let args = decode(
            &p,
            "mstudio_set_video_prompt",
            json!({"id":"s", "prompt":"English dialogue"}),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            args,
            json!({"action":"edit","operations":[{"op":"update_node","id":"s","shot":{"prompt":"English dialogue"}}]})
        );
        for invalid in [
            json!({"id":"s","patch":{"prompt":"x"}}),
            json!({"id":"s","shot":{"prompt":"x"}}),
            json!({"id":"s","prompt":"x","dialogue":"x"}),
        ] {
            assert!(
                decode(&p, "mstudio_set_video_prompt", invalid)
                    .unwrap()
                    .is_err()
            );
        }
        assert!(decode(&p, "mstudio_update_shot", json!({"id":"s","dialogue":"x"})).is_none());
    }
    #[test]
    fn generation_and_task_prompts_map_without_source_edits() {
        let p = profile("production");
        let args = decode(&p,"mstudio_generate_video",json!({"id":"s","prompt":"Final prompt","mode":"multi","parameters":{"duration":5,"resolution":"1080P"}})).unwrap().unwrap();
        assert_eq!(args["operations"][0]["op"], "request_generation");
        assert_eq!(args["operations"][0]["mediaKind"], "video");
        assert_eq!(args["operations"][0]["text"], "Final prompt");
        assert!(args["operations"][0].get("prompt").is_none());
        assert_eq!(
            decode(
                &p,
                "mstudio_update_generation",
                json!({"taskKey":"t","prompt":"Replacement"})
            )
            .unwrap()
            .unwrap(),
            json!({"action":"edit","operations":[{"op":"update_generation","taskKey":"t","text":"Replacement"}]})
        );
        assert!(
            decode(
                &p,
                "mstudio_generate_video",
                json!({"prompt":"x","mediaKind":"image"})
            )
            .unwrap()
            .is_err()
        );
        let asset = profile("asset-designer");
        assert!(decode(&asset, "mstudio_generate_video", json!({"prompt":"x"})).is_none());
        let args = decode(
            &asset,
            "mstudio_generate_reference_image",
            json!({"prompt":"x","references":[]}),
        )
        .unwrap()
        .unwrap();
        assert_eq!(args["operations"][0]["generationPurpose"], "asset");
    }
    #[test]
    fn every_role_has_unique_classified_tools_and_valid_internal_mapping() {
        for p in crate::assistant::profiles::builtins() {
            let mut names = std::collections::HashSet::new();
            for tool in tools(&p) {
                assert!(
                    names.insert(tool.definition.name.clone()),
                    "duplicate {}",
                    tool.definition.name
                );
                assert!(is_edit(&tool.definition.name), "{}", tool.definition.name);
                assert!(tool.definition.parameters["properties"].get("op").is_none());
                assert!(
                    tool.definition.parameters["properties"]
                        .get("operations")
                        .is_none()
                );
            }
        }
        assert!(is_edit("mstudio_edit")); // historical receipt recovery
        assert!(!is_edit("mstudio_inspect"));
        let mut p = profile("production");
        p.tool_ids = vec!["project-read".into(), "project-edit".into()];
        let args = decode(
            &p,
            "mstudio_update_shots",
            json!({"items":[{"id":"a","order":2},{"id":"b","order":1}]}),
        )
        .unwrap()
        .unwrap();
        assert_eq!(
            args,
            json!({"action":"edit","operations":[{"op":"update_node","id":"a","shot":{"order":2}},{"op":"update_node","id":"b","shot":{"order":1}}]})
        );
        assert!(super::super::schema::issues(&tool_schema::for_profile(&p), &args).is_empty());
        assert!(
            decode(&p, "mstudio_update_shots", json!({"items":[]}))
                .unwrap()
                .is_err()
        );
    }
    #[test]
    fn creation_tools_fix_kind_and_keep_body_separate_from_shot_fields() {
        for (role, name, input, kind) in [
            (
                "storyboard",
                "mstudio_add_shot",
                json!({"id":"new","title":"Shot","text":"Action","screenplayId":"script","scriptId":"paragraph","duration":5}),
                "shot",
            ),
            (
                "asset-designer",
                "mstudio_add_asset",
                json!({"id":"new","title":"Asset","assetId":"media"}),
                "asset",
            ),
        ] {
            let p = profile(role);
            let args = decode(&p, name, input).unwrap().unwrap();
            assert_eq!(args["operations"][0]["kind"], kind);
            assert!(
                super::super::schema::issues(&tool_schema::for_profile(&p), &args).is_empty(),
                "{args}"
            );
            if kind == "shot" {
                assert_eq!(args["operations"][0]["text"], "Action");
                assert_eq!(args["operations"][0]["shot"]["duration"], 5);
                assert!(args["operations"][0]["shot"].get("text").is_none());
            }
        }
    }
    #[test]
    fn direct_prompt_tool_commits_once_and_recovers_its_receipt() {
        use rig_core::message::{AssistantContent, Message, ToolCall, ToolFunction};
        let root = std::env::temp_dir().join(format!("mstudio-split-{}", mstudio::media::id()));
        let store = crate::database::Store::open(root.clone()).unwrap();
        let doc = json!({"id":"p","name":"Synthetic","revision":0,"brief":"","width":1280,"height":720,"fps":30,"assets":[],"nodes":[
            {"id":"script","kind":"screenplay","title":"Script","x":0,"y":0,"width":280,"height":218,"screenplay":{"script":[{"id":"paragraph","title":"Scene","action":"Action","duration":5}]}},
            {"id":"shot","kind":"shot","title":"Shot","text":"Preserved staging","x":320,"y":0,"width":280,"height":218,"shot":{"screenplayId":"script","scriptId":"paragraph","order":1,"duration":5,"dialogue":"Preserved dialogue","prompt":"Old"}}
        ],"clips":[],"tracks":[],"captions":[]});
        crate::projects::write_document(&store, doc, true).unwrap();
        let p = profile("production");
        crate::project_service::execute(
            &store,
            &p,
            "p",
            "turn",
            "read",
            json!({"action":"inspect","nodeIds":["shot","script"]}),
        )
        .unwrap();
        let input = json!({"id":"shot","prompt":"New video prompt"});
        let args = decode(&p, "mstudio_set_video_prompt", input.clone())
            .unwrap()
            .unwrap();
        let receipt =
            crate::project_service::execute(&store, &p, "p", "turn", "turn:call", args.clone())
                .unwrap();
        assert_eq!(receipt["outcome"], "committed", "{receipt}");
        assert_eq!(
            crate::project_service::execute(&store, &p, "p", "turn", "turn:call", args).unwrap(),
            receipt
        );
        let saved = crate::project_service::execute(
            &store,
            &p,
            "p",
            "turn",
            "verify",
            json!({"action":"inspect","nodeIds":["shot"],"fields":["prompt","dialogue","text"]}),
        )
        .unwrap()
        .to_string();
        for value in [
            "New video prompt",
            "Preserved dialogue",
            "Preserved staging",
        ] {
            assert!(saved.contains(value), "{saved}");
        }
        let call = ToolCall::from_wire(
            "call",
            ToolFunction {
                name: "mstudio_set_video_prompt".into(),
                arguments: input,
            },
        );
        let mut messages = vec![Message::Assistant {
            id: None,
            content: vec![AssistantContent::ToolCall(call)],
        }];
        super::super::session_recovery::repair(&store, "p", "turn", &mut messages);
        assert_eq!(messages.len(), 2);
        let restored = serde_json::to_string(&messages[1]).unwrap();
        assert!(restored.contains("committed"), "{restored}");
        assert!(!restored.contains("EFFECT_UNKNOWN"));
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
