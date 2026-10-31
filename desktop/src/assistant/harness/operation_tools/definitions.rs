#[path = "descriptions.rs"]
mod descriptions;
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
                "Create one task awaiting user confirmation using the complete prompt and actual references. Reusable reference images are standalone: create them directly, without a placeholder asset node or target ID. Never submits to the generation service. Update an existing task instead of creating a duplicate. Only the user can start generation. Generated reusable references are candidates until explicit asset confirmation; collecting them as materials does not approve them or remove source memberships.",
            );
        } else if op == "remove_concepts" {
            add(
                &mut out,
                "remove_concepts",
                op,
                operation.clone(),
                Default::default(),
                None,
                "Delete only the specified concept card IDs, including the last card. Use only when the user requests deletion. Rejected directions normally stay on the board with status=killed. Does not remove scripts, shots or media.",
            );
        } else if op == "save_concepts" {
            add(
                &mut out,
                "save_concepts",
                op,
                operation.clone(),
                Default::default(),
                None,
                "Save the project concept board. Each concept has title, oneLine, truth, brandIdea, participant, want, ritual, evidence, reframe, risk, optional cost and status (rough/developed/selected/killed). truth must be a specific human observation that survives without the product; brandIdea must be a worldview, not a feature or positioning line. A format (ASMR, POV, pack-with-me, three-ways) is not a concept. mode=merge upserts by id; mode=replace rewrites the board. This does not write a screenplay, shot or timeline.",
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
                // Screenplay content is structured; the domain rejects body text
                // even for the coordinator's unrestricted node permissions.
                if kind == "screenplay" {
                    properties.remove("text");
                }
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
                if let Some(mut fields) = fields {
                    if kind == "screenplay" {
                        fields.retain(|key, _| key == "script");
                    }
                    properties.extend(fields);
                }
                if kind == "shot" {
                    parameters["required"].as_array_mut().unwrap().extend([
                        json!("screenplayId"),
                        json!("order"),
                        json!("duration"),
                    ]);
                } else if kind == "asset" {
                    parameters["required"]
                        .as_array_mut()
                        .unwrap()
                        .push(json!("assetId"));
                }
                add(
                    &mut out,
                    &format!("add_{kind}"),
                    op,
                    parameters,
                    serde_json::Map::from_iter([("kind".into(), json!(kind))]),
                    nested,
                    &format!(
                        "Create a {kind} node with a new id and title. {} All listed fields are direct parameters. x/y, when supplied, are absolute canvas coordinates.",
                        if kind == "screenplay" {
                            "Only one screenplay is allowed per project; update an existing one with mstudio_update_screenplay. script contains film paragraphs in order. Supplementary notes belong in the saved Agent output, handed off via outputRef."
                        } else if kind == "asset" {
                            "assetId must identify existing project media; this only places that media on the canvas. To generate a missing reusable asset, use mstudio_generate_reference_image directly without creating a node."
                        } else {
                            ""
                        }
                    ),
                );
            }
        } else if op == "update_node" {
            if operation["properties"].get("shot").is_some() {
                for (name, fields, description) in [
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
                        serde_json::Map::from_iter([("expectedKind".into(), json!("shot"))]),
                        Some("shot"),
                        description,
                    );
                    if name == "update_shot" {
                        let source = out.last().unwrap();
                        out.push(OperationTool { definition: ToolDefinition { name:"mstudio_update_shots".try_into().expect("nonempty tool name"),description:"Atomically update multiple shots, including order swaps. Each items entry has the same direct fields as update_shot; no op or shot wrapper. The entire batch succeeds or fails together.".into(),parameters:json!({"type":"object","properties":{"items":{"type":"array","minItems":1,"maxItems":30,"items":source.definition.parameters}},"required":["items"],"additionalProperties":false}) },op:op.into(),fixed:serde_json::Map::from_iter([("expectedKind".into(), json!("shot"))]),nested:Some("shot".into()),task_prompt:false,batch:true });
                    }
                }
            }
            if operation["properties"].get("screenplay").is_some() {
                let mut parameters = flat(
                    operation,
                    "screenplay",
                    &["script", "removeParagraphIds", "paragraphOrder"],
                );
                parameters["properties"]["title"] = operation["properties"]["title"].clone();
                add(
                    &mut out,
                    "update_screenplay",
                    op,
                    parameters,
                    serde_json::Map::from_iter([("expectedKind".into(), json!("screenplay"))]),
                    Some("screenplay"),
                    "Update the existing screenplay's title or paragraphs by stable IDs. Omitted paragraphs and fields are preserved. Explicit removeParagraphIds deletes paragraphs; paragraphOrder must list all remaining IDs exactly once. For a rewrite, supply the retained/new paragraphs and explicitly remove obsolete IDs in this same call. Supplementary notes belong in the saved Agent output, not timed film paragraphs.",
                );
            }
            for (kind, name) in [
                ("shot", "update_shot_design"),
                ("asset", "update_asset_node"),
                ("note", "update_note"),
                ("text", "update_text"),
            ] {
                let mut node = operation.clone();
                node["properties"]
                    .as_object_mut()
                    .unwrap()
                    .retain(|key, _| {
                        let permitted = match kind {
                            "shot" => ["op", "id", "expectedKind", "title", "text"]
                                .contains(&key.as_str()),
                            "asset" => ["op", "id", "expectedKind", "title", "text", "assetId"]
                                .contains(&key.as_str()),
                            _ => ["op", "id", "expectedKind", "title", "text"]
                                .contains(&key.as_str()),
                        };
                        permitted
                            && crate::assistant::permissions::allows_node_field(profile, kind, key)
                    });
                if node["properties"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .all(|key| ["op", "id", "expectedKind"].contains(&key.as_str()))
                {
                    continue;
                }
                if let Some(text) = node["properties"].get_mut("text") {
                    text["description"] = json!(
                        "Body text. For shots this is action/staging; use dedicated prompt tools for image/video prompts."
                    );
                }
                add(
                    &mut out,
                    name,
                    op,
                    node,
                    serde_json::Map::from_iter([("expectedKind".into(), json!(kind))]),
                    None,
                    &format!(
                        "Update the {kind} node's direct title/body/asset properties. The ID must belong to a {kind}. Prompts, shot structure and screenplay paragraphs use their dedicated tools."
                    ),
                );
            }
        } else if op == "remove_node" {
            for (kind, name, capability) in [
                ("shot", "remove_shot", "project-shots"),
                ("screenplay", "remove_screenplay", "project-script"),
                ("asset", "remove_asset_node", "project-assets"),
                ("note", "remove_note", "project-edit"),
                ("text", "remove_text", "project-edit"),
            ] {
                if !profile
                    .tool_ids
                    .iter()
                    .any(|id| id == capability || id == "project-edit")
                {
                    continue;
                }
                add(
                    &mut out,
                    name,
                    op,
                    operation.clone(),
                    serde_json::Map::from_iter([("expectedKind".into(), json!(kind))]),
                    None,
                    &format!(
                        "Remove the {kind} canvas node identified by id. The ID must belong to a {kind}. This does not delete the underlying media asset."
                    ),
                );
            }
        } else if op == "update_clip" {
            super::clip_tools::add_updates(&mut out, operation);
        } else if op == "append_clip" {
            let mut parameters = operation.clone();
            parameters["properties"]
                .as_object_mut()
                .unwrap()
                .remove("visual");
            add(
                &mut out,
                "append_clip",
                op,
                parameters,
                Default::default(),
                None,
                "Append assetId to the timeline with optional placement and source range in seconds. Visual adjustments use set_clip_visual or set_clip_grade.",
            );
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
            let description = descriptions::description(op);
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
