use serde_json::{Value, json};
pub fn schema() -> Value {
    let operation: Value =
        serde_json::from_str(include_str!(concat!(env!("OUT_DIR"), "/operations.json")))
            .expect("compiled operation contract");
    json!({"type":"object","properties":{
      "action":{"type":"string","enum":["inspect","edit","history","skills","read_skill","models"]},
      "skill":{"type":"string","description":"read_skill directory ID from the skills catalog."},
      "path":{"type":"string","description":"Markdown path relative to the Skill root; default SKILL.md. Must stay within this Skill; cross-Skill paths are forbidden."},
      "section":{"type":"string","enum":["creation","captions","tracks","assets","clips","generation"],"description":"inspect section to read; omission returns a compact project summary."},
      "fields":{"type":"array","items":{"type":"string","enum":["title","shot","shot.order","shot.duration","shots","dialogue","screenplay","script","framePrompt","prompt","frames","takes","references","assetId","resultAssetId","shotId","start","trimIn","trimOut","speed","trackId","visual","volume","fadeIn","fadeOut","x","y","scale","opacity","transition","name","kind","duration","width","height","text","style","muted","hidden","status","targetNodeId","resultAssetIds","error","trackingPaused","turnId","modelId","ownerId","inputs","parameters","generationPurpose","source"]},"description":"With nodeIds, return selected fields plus id/kind. shot is basic structure; shot.order/shot.duration select order/timing; shots is a screenplay shot directory; text is action/staging. Omission returns summaries. script gives paragraph summaries; screenplay gives the full paginated script. Narrow with paragraphIds/scriptFields."},
      "ids":{"type":"array","maxItems":12,"items":{"type":"string"},"description":"inspect section=clips/assets/tracks/captions: exact IDs; assets include source with the original submitted prompt, model and references. fields=[source] selects provenance; source.prompt.nextTextOffset paginates original text."},
      "nodeIds":{"type":"array","maxItems":12,"items":{"type":"string"},"description":"inspect up to 12 nodes; select needed fields and paginate text with textOffset."},
      "paragraphIds":{"type":"array","maxItems":12,"items":{"type":"string"},"description":"inspect fields=script/screenplay: only these paragraphs; missing IDs are reported."},
      "scriptFields":{"type":"array","items":{"type":"string","enum":["title","duration","action","onScreenText","dialogue","sound"]},"description":"Paragraph fields; id always returned. script defaults to title/duration; screenplay defaults to all. Paginate text with textOffset."},
      "taskId":{"type":"string","description":"history: filter task history by taskScope.taskId."},
      "turnId":{"type":"string","description":"inspect section=generation: filter by durable batch/turn ID."},
      "status":{"type":"string","enum":["DRAFT","AWAITING_CONFIRMATION","READY","UPLOADING","SUBMITTING","IN_QUEUE","IN_PROGRESS","RECEIVING","COMPLETED","FAILED","CANCEL_REQUESTED","CANCELLED","UNKNOWN"],"description":"inspect section=generation: exact task status; batch reports whole-batch statistics."},
      "taskKey":{"type":"string","description":"inspect section=generation: exact task ID, including hidden records."},
      "messageId":{"type":"integer","description":"history: read this historical message."},
      "textOffset":{"type":"integer","description":"inspect/history text offset; use returned nextTextOffset."},
      "mediaModelId":{"type":"string","description":"models: selected generation model ID; returns model-specific prompt rules and parameter capabilities with exact fields, enums, bounds and defaults."},
      "offset":{"type":"integer","description":"inspect/history page offset, default 0."},
      "operations":{"type":"array","minItems":1,"maxItems":30,"items":operation}
    },"required":["action"],"additionalProperties":false})
}
/// Advertise only installed capabilities; dispatch still checks authorization.
pub fn for_profile(profile: &super::profiles::AgentProfile) -> Value {
    let mut schema = schema();
    schema["properties"]["action"]["enum"]
        .as_array_mut()
        .unwrap()
        .retain(|v| super::profiles::allows(profile, v.as_str().unwrap_or("")));
    if !super::profiles::allows(profile, "edit") {
        let p = schema["properties"].as_object_mut().unwrap();
        p.remove("operations");
        p.remove("revision");
    } else {
        let operations = schema["properties"]["operations"]["items"]["oneOf"]
            .as_array_mut()
            .unwrap();
        operations.retain(|op| {
            super::permissions::allows_operation(
                profile,
                op["properties"]["op"]["const"].as_str().unwrap_or(""),
            ) && (op["properties"]["op"]["const"] != "request_generation"
                || ((!super::permissions::asset_only(profile)
                    || op["properties"]["generationPurpose"]["const"] == "asset")
                    && super::permissions::allows_media_kind(
                        profile,
                        op["properties"]["mediaKind"]["const"]
                            .as_str()
                            .unwrap_or(""),
                    )))
        });
        for operation in operations {
            let properties = operation["properties"].as_object_mut().unwrap();
            let name = properties["op"]["const"].as_str().unwrap_or("").to_owned();
            if matches!(name.as_str(), "add_node" | "update_node") {
                properties.retain(|key, _| {
                    ["asset", "screenplay", "shot"]
                        .iter()
                        .any(|kind| super::permissions::allows_node_field(profile, kind, key))
                });
                if let Some(kinds) = properties
                    .get_mut("kind")
                    .and_then(|v| v["enum"].as_array_mut())
                {
                    kinds.retain(|kind| {
                        super::permissions::allows_node_field(
                            profile,
                            kind.as_str().unwrap_or(""),
                            "kind",
                        )
                    });
                }
            }
            if let Some(shot) = properties.get_mut("shot") {
                shot["properties"]
                    .as_object_mut()
                    .unwrap()
                    .retain(|key, _| super::permissions::allows_shot_field(profile, key));
                let keys: Vec<_> = shot["properties"]
                    .as_object()
                    .unwrap()
                    .keys()
                    .cloned()
                    .collect();
                shot["description"] = json!(format!(
                    "Nested shot changes. Writable fields for this role: {}. All other shot fields are read-only for this role even when visible in inspections. Report changes outside this scope to the caller for an authorized editor; do not put them in prompt as a substitute for editing the source field.",
                    keys.join(", ")
                ));
            }
            if name == "update_node" {
                let keys: Vec<_> = properties.keys().cloned().collect();
                let mut example = json!({"op":"update_node","id":"existing-node-id"});
                if let Some(shot) = properties.get("shot") {
                    let fields = shot["properties"].as_object().unwrap();
                    if let Some(field) = ["prompt", "framePrompt", "dialogue"]
                        .into_iter()
                        .find(|field| fields.contains_key(*field))
                    {
                        example["shot"] = json!({field: "Complete replacement text"});
                    }
                } else if properties.contains_key("title") {
                    example["title"] = json!("Updated title");
                }
                operation["description"] = json!(format!(
                    "Update an existing node using direct fields. Allowed operation fields: {}. Put changes beside op/id, with shot fields inside shot. There is no patch, changes or data wrapper. Minimal structure example: {}. Replace the example ID and text with actual values; use only listed fields. Omitted fields are preserved.",
                    keys.join(", "),
                    example
                ));
            }
        }
    }
    schema
}
