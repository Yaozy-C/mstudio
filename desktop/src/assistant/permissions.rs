use super::profiles::{AgentProfile, allows};
use anyhow::{Result, ensure};
use serde_json::Value;

pub fn asset_only(p: &AgentProfile) -> bool {
    let has = |id: &str| p.tool_ids.iter().any(|s| s == id);
    has("project-assets")
        && !has("project-frames")
        && !has("project-production")
        && !has("project-edit")
}

/// Media kinds follow capabilities, not role names; custom combined profiles remain valid.
pub fn allows_media_kind(p: &AgentProfile, kind: &str) -> bool {
    let has = |id: &str| p.tool_ids.iter().any(|s| s == id);
    has("media-generation")
        && (has("project-edit")
            || match kind {
                "image" => has("project-frames") || has("project-assets"),
                "video" => has("project-production"),
                _ => false,
            })
}

pub fn allows_operation(p: &AgentProfile, op: &str) -> bool {
    if !allows(p, "edit") {
        return false;
    }
    let has = |id: &str| p.tool_ids.iter().any(|s| s == id);
    if op == "update_generation" {
        return has("project-production")
            || has("project-frames")
            || has("project-assets")
            || has("project-edit");
    }
    if op == "request_generation" || op == "regenerate_generation" {
        return has("media-generation")
            && (has("project-production")
                || has("project-frames")
                || has("project-assets")
                || has("project-edit"));
    }
    if has("project-edit") {
        return true;
    }
    match op {
        "set_creation" | "set_brief" => has("project-brief"),
        "add_node" | "remove_node" => {
            has("project-script") || has("project-shots") || has("project-assets")
        }
        "update_node" => {
            has("project-script")
                || has("project-shots")
                || has("project-production")
                || has("project-frames")
                || has("project-assets")
        }
        "set_references" => {
            has("project-shots")
                || has("project-production")
                || has("project-frames")
                || has("project-assets")
        }
        "move_clip"
        | "retime_clip"
        | "slip_clip"
        | "set_transition"
        | "choose_take"
        | "assemble_screenplay"
        | "append_clip"
        | "update_clip"
        | "remove_clip"
        | "add_track"
        | "update_track"
        | "add_caption"
        | "update_caption"
        | "remove_caption" => has("project-timeline"),
        _ => false,
    }
}
pub fn allows_shot_field(p: &AgentProfile, key: &str) -> bool {
    let has = |id: &str| p.tool_ids.iter().any(|s| s == id);
    has("project-edit")
        || (has("project-shots")
            && [
                "screenplayId",
                "scriptId",
                "order",
                "duration",
                "dialogue",
                "frames",
            ]
            .contains(&key))
        || (has("project-production") && key == "prompt")
        || (has("project-frames") && ["framePrompt", "frames"].contains(&key))
}
/// Shared by the advertised node schema and the actual field authorization.
pub fn allows_node_field(p: &AgentProfile, kind: &str, key: &str) -> bool {
    let has = |id: &str| p.tool_ids.iter().any(|s| s == id);
    has("project-edit")
        || matches!(key, "op" | "id")
        || (kind == "asset"
            && has("project-assets")
            && ["kind", "title", "text", "assetId", "x", "y"].contains(&key))
        || (kind == "screenplay"
            && has("project-script")
            && ["kind", "title", "screenplay"].contains(&key))
        || (kind == "shot"
            && has("project-shots")
            && ["kind", "title", "text", "shot", "references", "x", "y"].contains(&key))
        || (kind == "shot"
            && (has("project-production") || has("project-frames"))
            && ["shot", "references"].contains(&key))
}

/// Check the complete batch before any project mutation, including field-level scope.
pub fn validate(p: &AgentProfile, args: &Value, doc: &Value) -> Result<()> {
    let ops = args["operations"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("Missing operations list"))?;
    let has = |id: &str| p.tool_ids.iter().any(|s| s == id);
    let mut kinds: std::collections::HashMap<String, String> = doc["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| Some((n["id"].as_str()?.into(), n["kind"].as_str()?.into())))
        .collect();
    for op in ops {
        let name = op["op"].as_str().unwrap_or("");
        if matches!(name, "update_generation" | "regenerate_generation") {
            let key = op["taskKey"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing task ID"))?;
            let task = &doc["production"]["drafts"][key];
            ensure!(task.is_object(), "Task not found");
            if asset_only(p) {
                ensure!(
                    task["generationPurpose"] == "asset",
                    "Asset Agent can only modify asset tasks"
                );
            }
            let kind = task["kind"].as_str().unwrap_or("");
            // Updating prompt text does not require generation permission.
            let mut author = p.clone();
            if !author.tool_ids.iter().any(|id| id == "media-generation") {
                author.tool_ids.push("media-generation".into());
            }
            ensure!(
                allows_media_kind(&author, kind),
                "Task media kind outside this Agent's scope"
            );
        }
        if name == "request_generation" {
            ensure!(
                allows_media_kind(p, op["mediaKind"].as_str().unwrap_or("")),
                "Generation media kind outside this Agent's scope"
            );
        }
        ensure!(
            allows_operation(p, name),
            "Operation not permitted for this Agent: {name}"
        );
        if has("project-edit") {
            continue;
        }
        if matches!(
            name,
            "add_node" | "update_node" | "remove_node" | "set_references"
        ) {
            let id = op["id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Missing node ID"))?;
            let kind = if name == "add_node" {
                op["kind"].as_str().unwrap_or("")
            } else {
                kinds.get(id).map(String::as_str).unwrap_or("")
            };
            // Creating or deleting a record is structural and belongs to the
            // capability that owns that kind of record. Editing an existing
            // record's content may also be done by the roles that fill in its
            // prompt, frames or references.
            let structural = matches!(name, "add_node" | "remove_node");
            let screenplay = kind == "screenplay" && has("project-script");
            let shot = kind == "shot"
                && if structural {
                    has("project-shots")
                } else {
                    has("project-shots") || has("project-production") || has("project-frames")
                };
            let asset = kind == "asset" && has("project-assets");
            let references = kind == "shot"
                && name == "set_references"
                && (has("project-shots")
                    || has("project-production")
                    || has("project-frames")
                    || has("project-assets"));
            ensure!(
                screenplay || shot || asset || references,
                "Node outside this Agent's edit scope"
            );
            if name == "add_node" || name == "update_node" {
                let fields = op
                    .as_object()
                    .ok_or_else(|| anyhow::anyhow!("Invalid operation format"))?;
                for key in fields.keys().map(String::as_str) {
                    let allowed = allows_node_field(p, kind, key);
                    ensure!(allowed, "Field not editable by this Agent: {key}");
                }
                if let Some(fields) = op.get("shot") {
                    let fields = fields
                        .as_object()
                        .ok_or_else(|| anyhow::anyhow!("shot must be an object"))?;
                    for key in fields.keys().map(String::as_str) {
                        ensure!(
                            allows_shot_field(p, key),
                            "Shot field not editable by this Agent: {key}"
                        );
                    }
                }
            }
            if name == "add_node" {
                kinds.insert(id.into(), kind.into());
            } else if name == "remove_node" {
                kinds.remove(id);
            }
        }
    }
    Ok(())
}
#[cfg(test)]
#[path = "permissions_tests.rs"]
mod tests;
