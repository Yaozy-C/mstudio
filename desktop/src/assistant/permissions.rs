use super::profiles::{AgentProfile, allows};
use anyhow::{Result, ensure};
use serde_json::Value;

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
/// Check the complete batch before any UI mutation, including field-level scope.
pub fn validate(p: &AgentProfile, args: &Value, doc: &Value) -> Result<()> {
    let ops = args["operations"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("缺少操作列表"))?;
    let has = |id: &str| p.tool_ids.iter().any(|s| s == id);
    let mut kinds: std::collections::HashMap<String, String> = doc["nodes"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|n| Some((n["id"].as_str()?.into(), n["kind"].as_str()?.into())))
        .collect();
    for op in ops {
        let name = op["op"].as_str().unwrap_or("");
        let asset_only = has("project-assets")
            && !has("project-frames")
            && !has("project-production")
            && !has("project-edit");
        if asset_only && name == "request_generation" {
            ensure!(
                op["generationPurpose"] == "asset"
                    && op.get("id").is_none()
                    && op.get("canvasTaskKey").is_none(),
                "资产 Agent 只能创建独立资产任务"
            );
        }
        if matches!(name, "update_generation" | "regenerate_generation") {
            let key = op["taskKey"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少任务 ID"))?;
            let task = &doc["production"]["drafts"][key];
            ensure!(task.is_object(), "任务不存在");
            if asset_only {
                ensure!(
                    task["generationPurpose"] == "asset",
                    "资产 Agent 只能修改资产任务"
                );
            }
            if !has("project-production") && !has("project-edit") {
                ensure!(task["kind"] == "image", "当前 Agent 只能修改图片任务");
            }
        }
        ensure!(
            allows_operation(p, name),
            "当前 Agent 没有此操作能力：{name}"
        );
        if has("project-edit") {
            continue;
        }
        if name == "request_generation" && !has("project-production") {
            ensure!(op["mediaKind"] == "image", "当前 Agent 只能生成图片");
        }
        if matches!(
            name,
            "add_node" | "update_node" | "remove_node" | "set_references"
        ) {
            let id = op["id"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少节点 ID"))?;
            let kind = if name == "add_node" {
                op["kind"].as_str().unwrap_or("")
            } else {
                kinds.get(id).map(String::as_str).unwrap_or("")
            };
            let screenplay = kind == "screenplay" && has("project-script");
            let shot = kind == "shot" && has("project-shots");
            let production = kind == "shot" && has("project-production");
            let frames = kind == "shot" && has("project-frames");
            let asset = has("project-assets")
                && (kind == "asset" || (kind == "shot" && name == "set_references"));
            ensure!(
                screenplay || shot || production || frames || asset,
                "此节点不属于当前 Agent 的编辑范围"
            );
            if name == "add_node" || name == "update_node" {
                let fields = op
                    .as_object()
                    .ok_or_else(|| anyhow::anyhow!("操作格式无效"))?;
                for key in fields.keys().map(String::as_str) {
                    let allowed = matches!(key, "op" | "id")
                        || (asset && ["kind", "title", "text", "assetId", "x", "y"].contains(&key))
                        || (screenplay && ["kind", "title", "screenplay"].contains(&key))
                        || (shot
                            && ["kind", "title", "text", "shot", "references", "x", "y"]
                                .contains(&key))
                        || ((production || frames) && ["shot", "references"].contains(&key));
                    ensure!(allowed, "当前 Agent 不能修改字段：{key}");
                }
                if let Some(fields) = op.get("shot") {
                    let fields = fields
                        .as_object()
                        .ok_or_else(|| anyhow::anyhow!("镜头字段必须是对象"))?;
                    for key in fields.keys().map(String::as_str) {
                        ensure!(
                            allows_shot_field(p, key),
                            "当前 Agent 不能修改镜头字段：{key}"
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
