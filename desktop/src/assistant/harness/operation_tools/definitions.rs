use super::{OperationTool, add, flat};
use crate::assistant::{profiles::AgentProfile, tool_schema};
use rig_core::completion::ToolDefinition;
use serde_json::{Value, json};

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
                description,
            );
        }
    }
    out
}
