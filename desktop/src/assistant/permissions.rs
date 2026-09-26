use super::profiles::{AgentProfile, allows};
use anyhow::{Result, ensure};
use serde_json::Value;

pub fn allows_operation(p: &AgentProfile, op: &str) -> bool {
    if !allows(p, "edit") {
        return false;
    }
    let has = |id: &str| p.tool_ids.iter().any(|s| s == id);
    if op == "update_generation" {
        return has("project-production") || has("project-frames") || has("project-edit");
    }
    if op == "request_generation" || op == "regenerate_generation" {
        return has("media-generation")
            && (has("project-production") || has("project-frames") || has("project-edit"));
    }
    if has("project-edit") {
        return true;
    }
    match op {
        "set_creation" | "set_brief" => has("project-brief"),
        "add_node" | "remove_node" => has("project-plan") || has("project-shots"),
        "update_node" => {
            has("project-plan")
                || has("project-shots")
                || has("project-production")
                || has("project-frames")
        }
        "set_references" => {
            has("project-shots") || has("project-production") || has("project-frames")
        }
        "choose_take" | "assemble_plan" | "append_clip" | "update_clip" | "remove_clip"
        | "add_track" | "update_track" | "add_caption" | "update_caption" | "remove_caption" => {
            has("project-timeline")
        }
        _ => false,
    }
}
pub fn allows_shot_field(p: &AgentProfile, key: &str) -> bool {
    let has = |id: &str| p.tool_ids.iter().any(|s| s == id);
    has("project-edit")
        || (has("project-shots")
            && [
                "planId", "scriptId", "order", "duration", "dialogue", "frames",
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
        if matches!(name, "update_generation" | "regenerate_generation") {
            let key = op["taskKey"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("缺少任务 ID"))?;
            let task = &doc["production"]["drafts"][key];
            ensure!(task.is_object(), "任务不存在");
            if !has("project-production") && !has("project-edit") {
                ensure!(task["kind"] == "image", "分镜画手只能修改图片任务");
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
            ensure!(op["mediaKind"] == "image", "分镜画手只能生成图片");
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
            let plan = kind == "plan" && has("project-plan");
            let shot = kind == "shot" && has("project-shots");
            let production = kind == "shot" && has("project-production");
            let frames = kind == "shot" && has("project-frames");
            ensure!(
                plan || shot || production || frames,
                "此节点不属于当前 Agent 的编辑范围"
            );
            if name == "add_node" || name == "update_node" {
                let fields = op
                    .as_object()
                    .ok_or_else(|| anyhow::anyhow!("操作格式无效"))?;
                for key in fields.keys().map(String::as_str) {
                    let allowed = matches!(key, "op" | "id")
                        || (plan && ["kind", "title", "text", "plan", "x", "y"].contains(&key))
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
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn task_edits_use_existing_media_roles_and_actual_task_kind() {
        let agents = super::super::profiles::builtins();
        let doc = json!({"nodes":[],"production":{"drafts":{"image":{"kind":"image"},"video":{"kind":"video"}}}});
        for operation in ["update_generation", "regenerate_generation"] {
            for (agent, key, allowed) in [
                ("production", "video", true),
                ("storyboard-artist", "image", true),
                ("storyboard-artist", "video", false),
                ("reviewer", "image", false),
                ("production", "missing", false),
            ] {
                let result = validate(
                    agents.iter().find(|a| a.id == agent).unwrap(),
                    &json!({"operations":[{"op":operation,"taskKey":key,"text":"new"}]}),
                    &doc,
                );
                assert_eq!(result.is_ok(), allowed, "{agent}: {operation} {key}");
            }
        }
    }
    #[test]
    fn specialists_cannot_cross_edit_boundaries() {
        let agents = super::super::profiles::builtins();
        let doc = json!({"nodes":[{"id":"p","kind":"plan"},{"id":"s","kind":"shot"}]});
        let check = |id: &str, op: Value| {
            validate(
                agents.iter().find(|a| a.id == id).unwrap(),
                &json!({"operations":[op]}),
                &doc,
            )
            .is_ok()
        };
        assert!(check(
            "concept",
            json!({"op":"update_node","id":"p","text":"concept"})
        ));
        assert!(check(
            "concept",
            json!({"op":"update_node","id":"p","plan":{"script":[{"id":"para","action":"Story"}]}})
        ));
        assert!(check(
            "storyboard",
            json!({"op":"update_node","id":"s","shot":{"scriptId":"para"}})
        ));
        assert!(!check(
            "production",
            json!({"op":"update_node","id":"s","shot":{"framePrompt":"Image description"}})
        ));
        assert!(!check(
            "production",
            json!({"op":"update_node","id":"s","shot":{"frames":[]}})
        ));
        assert!(check(
            "storyboard-artist",
            json!({"op":"update_node","id":"s","shot":{"framePrompt":"Image description","frames":[]}})
        ));
        assert!(!check(
            "production",
            json!({"op":"update_node","id":"s","shot":{"scriptId":"other"}})
        ));
        assert!(!check(
            "concept",
            json!({"op":"update_node","id":"s","text":"rewrite"})
        ));
        assert!(check(
            "storyboard",
            json!({"op":"update_node","id":"s","shot":{"dialogue":"hello"}})
        ));
        assert!(!check(
            "storyboard",
            json!({"op":"request_generation","id":"s"})
        ));
        assert!(!check(
            "storyboard",
            json!({"op":"request_generation","id":"s"})
        ));
        assert!(check(
            "production",
            json!({"op":"request_generation","id":"s","text":"Close-up","mediaKind":"image"})
        ));
        assert!(check(
            "production",
            json!({"op":"update_node","id":"s","shot":{"prompt":"generate"}})
        ));
        assert!(!check(
            "production",
            json!({"op":"update_node","id":"s","text":"remove main action"})
        ));
        assert!(!check(
            "production",
            json!({"op":"update_node","id":"s","shot":{"duration":1}})
        ));
        assert!(!check(
            "production",
            json!({"op":"append_clip","assetId":"a"})
        ));
        assert!(check(
            "editor",
            json!({"op":"update_clip","id":"c","speed":1.5})
        ));
        assert!(!check(
            "reviewer",
            json!({"op":"update_clip","id":"c","speed":1.5})
        ));
        assert!(check(
            "coordinator",
            json!({"op":"set_creation","essential":"must see entry"})
        ));
        assert!(!check(
            "coordinator",
            json!({"op":"update_node","id":"p","text":"rewrite"})
        ));
    }
}
