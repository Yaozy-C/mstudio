//! Resolve durable references independently of the delegating model's prose.
use crate::{assistant::attachments::Reference, database::Store};
use serde::Deserialize;
use serde_json::{Value, json};

#[derive(Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "camelCase", deny_unknown_fields)]
enum Artifact {
    Node {
        id: String,
    },
    Asset {
        id: String,
    },
    Clip {
        id: String,
    },
    Generation {
        id: String,
    },
    Agent {
        id: String,
        #[serde(rename = "turnId")]
        turn_id: String,
    },
}

pub fn schema() -> Value {
    json!({"type":"array","maxItems":24,"description":"Saved project sources: node/asset/clip IDs, generation task keys (id=taskKey), or settled agent outputRefs. Original user attachments and task text are forwarded by the host. Supplementary specialist notes use the agent outputRef, not a screenplay paragraph.","items":{"oneOf":[
        {"type":"object","properties":{"kind":{"type":"string","enum":["node","asset","clip","generation"]},"id":{"type":"string","minLength":1,"maxLength":200}},"required":["kind","id"],"additionalProperties":false},
        {"type":"object","properties":{"kind":{"type":"string","const":"agent"},"id":{"type":"string","minLength":1,"maxLength":200},"turnId":{"type":"string","minLength":1,"maxLength":200}},"required":["kind","id","turnId"],"additionalProperties":false}
    ]}})
}

pub struct Resolved {
    pub attachments: Vec<Reference>,
    pub manifest: Value,
}

pub fn resolve(
    store: &Store,
    project: &str,
    doc: &Value,
    raw: Option<&Value>,
) -> Result<Resolved, String> {
    let raw = raw.cloned().unwrap_or_else(|| json!([]));
    let issues = super::schema::issues(&schema(), &raw);
    if !issues.is_empty() {
        return Err(super::schema::rejection(issues).to_string());
    }
    let artifacts: Vec<Artifact> = serde_json::from_value(raw).map_err(|e| e.to_string())?;
    let mut attachments = vec![];
    let mut manifest = vec![];
    for artifact in artifacts {
        let (kind, id, collection) = match artifact {
            Artifact::Node { id } => ("node", id, "nodes"),
            Artifact::Asset { id } => ("asset", id, "assets"),
            Artifact::Clip { id } => ("clip", id, "clips"),
            Artifact::Generation { id } => {
                if doc["production"]["drafts"]
                    .as_object()
                    .and_then(|drafts| drafts.get(&id))
                    .is_none_or(|task| {
                        task["key"] != id
                            || task["turnId"].as_str().is_none_or(str::is_empty)
                            || task["createdAt"].as_f64().is_none_or(|v| v <= 0.0)
                    })
                {
                    return Err("Referenced generation task is unavailable in this project".into());
                }
                let task = &doc["production"]["drafts"][&id];
                if task["hiddenFromList"] == true {
                    manifest.push(json!({"reference":{"kind":"generation","id":id},"savedTask":crate::assistant::archive_context::summary(doc, task)}));
                    continue;
                }
                let mut saved: serde_json::Map<String, Value> = [
                    "key",
                    "kind",
                    "status",
                    "targetNodeId",
                    "ownerId",
                    "modelId",
                    "mode",
                    "parameters",
                    "resultAssetId",
                    "resultAssetIds",
                    "error",
                ]
                .into_iter()
                .filter_map(|key| task.get(key).map(|v| (key.into(), v.clone())))
                .collect();
                for asset in task["resultAssetIds"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .chain(std::iter::once(&task["resultAssetId"]))
                    .filter_map(Value::as_str)
                {
                    if doc["assets"].as_array().is_some_and(|assets| {
                        assets
                            .iter()
                            .any(|a| a["id"] == asset && a["discarded"] != true)
                    }) && !attachments
                        .iter()
                        .any(|r: &Reference| r.kind == "asset" && r.id == asset)
                    {
                        attachments.push(Reference {
                            kind: "asset".into(),
                            id: asset.into(),
                        });
                    }
                }
                let usable =
                    crate::assistant::archive_context::summary(doc, task)["resultAssetIds"].clone();
                if !usable
                    .as_array()
                    .is_some_and(|ids| ids.contains(&task["resultAssetId"]))
                {
                    saved.remove("resultAssetId");
                }
                saved.insert("resultAssetIds".into(), usable);
                manifest.push(json!({"reference":{"kind":"generation","id":id},"savedTask":saved,"readWith":{"tool":"mstudio_read_generation","arguments":{"taskKey":id}},"authority":"Use actual result assets and task metadata. Read fields=[prompt] only for an explicit user request to edit the prompt, retry unchanged or investigate generation. A generated result does not establish product facts or authorize another generation."}));
                continue;
            }
            Artifact::Agent { id, turn_id } => {
                let value = super::agent_artifacts::saved(
                    &store.db.lock().unwrap(),
                    project,
                    &id,
                    &turn_id,
                )
                .map_err(|error| {
                    if super::read_failure::missing(&error) {
                        "Agent artifact is unavailable in this project or has not settled"
                            .to_owned()
                    } else {
                        format!("Cannot read saved Agent artifact: {error}")
                    }
                })?;
                manifest.push(json!({"reference":super::agent_artifacts::reference(&id,&turn_id),"status":value["status"],"stopReason":value["stopReason"],"readWith":{"tool":"mstudio_read_agent_result","arguments":{"id":id,"turnId":turn_id}},"authority":"Specialist-authored output and execution receipts; not a user statement or proof of product construction."}));
                continue;
            }
        };
        let item = if kind == "asset" {
            crate::previs::media::find(doc, &id)
        } else {
            doc[collection]
                .as_array()
                .and_then(|items| items.iter().find(|v| v["id"] == id))
        }
        .ok_or_else(|| format!("Referenced {kind} {id} is unavailable in project {project}; use the source's actual reference kind and ID"))?;
        let asset_id = if kind == "asset" {
            Some(id.as_str())
        } else {
            item["resultAssetId"].as_str().or(item["assetId"].as_str())
        };
        if asset_id.is_some_and(|id| {
            doc["assets"]
                .as_array()
                .is_some_and(|a| a.iter().any(|a| a["id"] == id && a["discarded"] == true))
        }) {
            return Err("Discarded result cannot be delegated as a reference".into());
        }
        let reader = match kind {
            "asset" => json!({"tool":"mstudio_read_assets","arguments":{"ids":[id]}}),
            "clip" => json!({"tool":"mstudio_read_clips","arguments":{"ids":[id]}}),
            _ if item["kind"] == "shot" => {
                json!({"tool":"mstudio_read_shots","arguments":{"ids":[id]}})
            }
            _ if item["kind"] == "screenplay" => {
                json!({"tool":"mstudio_read_screenplay","arguments":{"id":id}})
            }
            _ => json!({"tool":"mstudio_read_nodes","arguments":{"ids":[id]}}),
        };
        let mut source = json!({"reference":{"kind":kind,"id":id},"readWith":reader,"authority":"Saved project source; consult the original content, not a relay's description. Project objects remain live; agent outputs are pinned to their turn."});
        if kind == "node" && item["kind"] == "shot" {
            source["savedWork"] = super::saved_work::shot_state(doc, item);
            source["savedReferences"] = item["references"].clone();
        }
        manifest.push(source);
        attachments.push(Reference {
            kind: kind.into(),
            id,
        });
    }
    Ok(Resolved {
        attachments,
        manifest: json!(manifest),
    })
}

pub fn user_refs(original: &Value) -> Vec<Reference> {
    original["refs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            Some(Reference {
                kind: r["kind"].as_str()?.into(),
                id: r["id"].as_str()?.into(),
            })
        })
        .collect()
}

pub fn source_message(task: &str, agent: &str, turn: &str, manifest: &Value) -> String {
    format!(
        "Delegated work from Agent {agent} in turn {turn} (not an original user statement):\n{task}\n\nHost-resolved source artifacts:\n{manifest}\nUse result media and necessary task metadata, not old generation prompts. Read fields=[prompt] from a generation task or fields=[source] from an asset only when the user explicitly requests a prompt edit, unchanged retry or generation investigation. Read actual pixels when needed for the current decision; do not restart completed research. Relay descriptions do not replace saved artifacts. Product geometry comes from original product images, never an agent's interpretation or a generated image's source prompt."
    )
}

/// Forward explicit user attachments; specialists open saved artifacts on demand.
pub fn message(
    store: &Store,
    doc: &Value,
    model: &crate::assistant::config::Profile,
    role: &crate::assistant::profiles::AgentProfile,
    task: &str,
    source: &Value,
) -> Result<rig_core::message::Message, String> {
    let original = &source["originalRequest"];
    let mut text = format!(
        "Original user request (provided directly by the host): {}\n",
        original["production"]["instruction"]
            .as_str()
            .unwrap_or_default()
    );
    if role.id != "video-analyst"
        && let Some(scope) = crate::assistant::task_context::saved(
            store,
            doc["id"].as_str().ok_or("Missing project ID")?,
            source["senderTurnId"]
                .as_str()
                .ok_or("Missing sending turn")?,
        )?
        && scope.original_instruction
            != original["production"]["instruction"]
                .as_str()
                .unwrap_or_default()
    {
        text.push_str(&format!(
            "Original task context (current user requirements take precedence): {}\n",
            scope.original_instruction
        ));
    }
    if role.id != "video-analyst" && original["production"].is_object() {
        text.push_str(&format!("User-selected production settings: {}\n", json!({"models":original["production"]["models"],"parameters":original["production"]["task"]["parameters"]})));
    }
    text.push_str(&source_message(
        task,
        source["senderAgentId"].as_str().unwrap_or(""),
        source["senderTurnId"].as_str().unwrap_or(""),
        &source["artifacts"],
    ));
    text.push_str(&format!(
        "\nRequired generation task receipts: {}",
        source["expectedGenerations"]
    ));
    let refs: Vec<Reference> = if role.id == "video-analyst" {
        // This isolated role has no tools and must receive the original video.
        serde_json::from_value(source.get("refs").cloned().unwrap_or_else(|| json!([])))
            .map_err(|e| e.to_string())?
    } else {
        user_refs(original)
    };
    let payload = if role.id == "video-analyst"
        && source["attachmentsAlreadyProvided"] == true
        && refs.is_empty()
    {
        crate::assistant::attachments::payload(store, doc, &text, &refs, model)
            .map_err(|e| e.to_string())?
    } else {
        crate::assistant::turn_context::payload(store, doc, &text, &refs, model, role)?
    };
    Ok(crate::assistant::agent::convert(
        &json!({"role":"user","content":payload}),
    ))
}

#[cfg(test)]
#[path = "handoff_artifacts_tests.rs"]
mod tests;
