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
    json!({"type":"array","maxItems":24,"description":"Saved source artifacts to read directly, not paraphrase into task/message. Use node/asset/clip IDs, generation task keys (kind=generation, id=taskKey), or an agent outputRef from a settled run. Original user attachments are supplied separately by the host.","items":{"oneOf":[
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
                manifest.push(json!({"reference":{"kind":"generation","id":id},"projectRevision":doc["revision"],"savedTask":doc["production"]["drafts"][&id],"readWith":{"tool":"mstudio_read_generation","arguments":{"taskKey":id}},"authority":"Live saved generation task. Read its prompt, inputs, parameters and status directly; a generated result does not establish product facts or authorize another generation."}));
                continue;
            }
            Artifact::Agent { id, turn_id } => {
                let value = super::agent_artifacts::saved(
                    &store.db.lock().unwrap(),
                    project,
                    &id,
                    &turn_id,
                )
                .map_err(|_| {
                    "Agent artifact is unavailable in this project or has not settled".to_owned()
                })?;
                manifest.push(json!({"reference":super::agent_artifacts::reference(&id,&turn_id),"status":value["status"],"stopReason":value["stopReason"],"readWith":{"tool":"mstudio_read_agent_result","arguments":{"id":id,"turnId":turn_id}},"authority":"Specialist-authored output and execution receipts; not a user statement or proof of product construction."}));
                continue;
            }
        };
        let item = doc[collection]
            .as_array()
            .and_then(|items| items.iter().find(|v| v["id"] == id))
            .ok_or_else(|| format!("Referenced {kind} is unavailable in this project"))?;
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
        let mut source = json!({"reference":{"kind":kind,"id":id},"projectRevision":doc["revision"],"readWith":reader,"authority":"Saved project source; consult the original content, not a relay's description. Project objects remain live; agent outputs are pinned to their turn."});
        if kind == "node" && item["kind"] == "shot" {
            source["savedWork"] = super::saved_work::shot_state(doc, item);
            source["savedVideoPrompt"] = item["shot"]["prompt"].clone();
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
        "Delegated work from Agent {agent} in turn {turn} (not an original user statement):\n{task}\n\nHost-resolved source artifacts:\n{manifest}\nReuse complete saved prompts and task data supplied in this manifest. Read referenced outputs only for missing content, changed state, or pixels needed for the current decision; do not restart completed research. Relay descriptions do not replace saved artifacts. Product geometry comes from original product images, never an agent's interpretation or a generated image's source prompt."
    )
}

#[cfg(test)]
#[path = "handoff_artifacts_tests.rs"]
mod tests;
