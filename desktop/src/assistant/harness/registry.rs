use super::Host;
use crate::assistant::{memory::MemoryTool, profiles, tool_schema, tools::ProjectTool};
use rig_agent::{prelude::Tool, tool::ToolContext};
use rig_core::{completion::ToolDefinition, message::ToolCall};
use serde_json::{Value, json};
use tauri::Manager;
use tokio_util::sync::CancellationToken;

pub struct ProjectHost {
    pub tool: Option<ProjectTool>,
    pub media_profile: crate::assistant::config::Profile,
    pub token: CancellationToken,
    pub delegation: Option<super::delegation::Context>,
}
const ACTIONS: &[(&str, &str, &[&str])] = &[
    (
        "inspect",
        "读取当前任务所需工程内容；生成任务用 section=generation 与 taskKey 精确查询，镜头用 nodeIds/fields。按返回偏移补读。revision 是返回值，不是查询参数；修改前核实目标最新状态。",
        &[
            "section",
            "taskKey",
            "nodeIds",
            "fields",
            "offset",
            "textOffset",
        ],
    ),
    (
        "read_skill",
        "读取已启用技能的主规则或引用文件；独立文件请同批调用，nextOffset 分页须按顺序继续。",
        &["skill", "path", "offset"],
    ),
    ("skills", "列出当前 Agent 已启用的技能与文件目录。", &[]),
    (
        "history",
        "分页检索项目历史对话。",
        &["offset", "messageId", "textOffset"],
    ),
    ("models", "分页查看已启用的媒体模型。", &["offset"]),
    ("edit", "", &["revision", "operations"]),
];

impl ProjectHost {
    pub async fn read_image(&self, call: &ToolCall) -> Value {
        let Some(t) = &self.tool else {
            return json!({"error":"工具不可用"});
        };
        if !profiles::allows(&t.profile, "inspect") {
            return json!({"error":"没有读取工程权限","code":"FORBIDDEN"});
        }
        let store = t.app.state::<crate::database::Store>();
        let _files = store.files.read().await;
        super::image_read::read(
            &store,
            &t.project,
            &self.media_profile,
            &call.function.arguments,
        )
    }

    fn action<'a>(&self, name: &'a str) -> Option<&'a str> {
        let action = name.strip_prefix("mstudio_")?;
        ACTIONS
            .iter()
            .any(|(a, _, _)| *a == action)
            .then_some(action)
    }
}
impl Host for ProjectHost {
    fn token(&self) -> &CancellationToken {
        &self.token
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        match &self.tool {
            Some(t) => t.record(kind, value),
            None => Ok(()),
        }
    }
    fn definitions(&self) -> Vec<ToolDefinition> {
        let Some(t) = &self.tool else { return vec![] };
        let schema = tool_schema::for_profile(&t.profile);
        let mut definitions = Vec::new();
        for (action, description, fields) in ACTIONS {
            if !profiles::allows(&t.profile, action) {
                continue;
            }
            let properties: serde_json::Map<_, _> = fields
                .iter()
                .map(|field| ((*field).to_owned(), schema["properties"][*field].clone()))
                .collect();
            let required = match *action {
                "edit" => vec!["revision", "operations"],
                "read_skill" => vec!["skill"],
                _ => vec![],
            };
            definitions.push(ToolDefinition {
                name: format!("mstudio_{action}"),
                description: if *action == "edit" { edit_description() } else { (*description).into() },
                parameters: json!({"type":"object","properties":properties,"required":required,"additionalProperties":false}),
            });
        }
        if profiles::allows(&t.profile, "memory-read") {
            let memory = MemoryTool(t.clone());
            definitions.push(ToolDefinition {
                name: "mstudio_memory".into(),
                description: memory.description(),
                parameters: memory.parameters(),
            });
        }
        if profiles::allows(&t.profile, "inspect") {
            definitions.push(ToolDefinition {
                name: "mstudio_read_image".into(),
                description: "读取当前项目真实图片像素（含刚生成的图片）。assetId 使用 inspect 的 assets、frames 或 generation 返回的真实素材 ID，不是任务 ID 或文件名。当前模型必须支持图片输入。".into(),
                parameters: json!({"type":"object","properties":{"assetId":{"type":"string"}},"required":["assetId"],"additionalProperties":false}),
            });
            definitions.push(ToolDefinition {
                name: "mstudio_reopen_image".into(),
                description: "按卸载提示中的 imageId 重新读取历史图像。仅可读取当前工程已卸载的图像；图像将作为图片工具结果返回。".into(),
                parameters: json!({"type":"object","properties":{"imageId":{"type":"string"}},"required":["imageId"],"additionalProperties":false}),
            });
        }
        if profiles::allows(&t.profile, "delegate") {
            let mut definition = super::delegation::definition();
            let catalog =
                profiles::read(&t.app.state::<crate::database::Store>().db.lock().unwrap())
                    .unwrap_or_default();
            let ids: Vec<_> = catalog
                .iter()
                .filter(|p| p.enabled && p.id != t.profile.id && p.id != "coordinator")
                .map(|p| p.id.clone())
                .collect();
            definition.parameters["properties"]["agentId"]["enum"] = json!(ids);
            let roster: Vec<_> = catalog
                .iter()
                .filter(|p| ids.contains(&p.id))
                .map(|p| json!({"id":p.id,"tools":p.tool_ids}))
                .collect();
            definition.description.push_str(&format!("可用角色与权限：{}。交付 script 用 project-plan，shots 用 project-shots，image-prompts/images 用 project-frames，video-prompts/videos 用 project-production，timeline 用 project-timeline；生成还需 media-generation。",json!(roster)));

            definitions.push(definition);
            definitions.extend(super::delegation::control_definitions());
        }
        definitions
    }
    fn parallel_safe(&self, call: &ToolCall) -> bool {
        // Project reads go through the UI edit queue and are ordering barriers.
        // Skill files/history/catalog are independent reads; memory mutations are exclusive.
        matches!(
            self.action(&call.function.name),
            Some("read_skill" | "skills" | "history" | "models")
        ) || (call.function.name == "mstudio_memory" && call.function.arguments["action"] == "list")
            || call.function.name == "mstudio_reopen_image"
            || matches!(
                call.function.name.as_str(),
                "mstudio_send_message" | "mstudio_interrupt_agent" | "mstudio_list_agents"
            )
    }
    fn injected(&self) -> Result<Vec<rig_core::message::Message>, String> {
        self.tool
            .as_ref()
            .filter(|t| profiles::allows(&t.profile, "delegate"))
            .map_or(Ok(vec![]), super::delegation::notices)
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        let Some(t) = &self.tool else {
            return json!({"error":"工具不可用", "code":"UNKNOWN_TOOL"});
        };
        if call.function.name == "mstudio_delegate" {
            return super::delegation::execute(self, call).await;
        }
        if matches!(
            call.function.name.as_str(),
            "mstudio_send_message" | "mstudio_interrupt_agent" | "mstudio_list_agents"
        ) {
            return super::delegation::execute_control(self, call).await;
        }
        if call.function.name == "mstudio_read_image" {
            return self.read_image(call).await;
        }
        execute_local(t, call, "").await
    }
}

fn edit_description() -> String {
    "按当前角色权限批量修改工程；revision 使用 inspect 返回值，操作原子保存且可撤销。只修改本轮目标，已有对象沿用 ID；add_node 必填 id/kind/title，shot 关联 planId；省略字段保留原值。update_node 更新卡片：plan.script 为脚本，镜头 text 为动作与摄影设计，shot.prompt 为视频草稿，shot.framePrompt/frames 为画格草稿与图片。修改已有生成任务用 update_generation(taskKey,text)，不顺带覆盖镜头草稿；已提交任务仅保存 nextPrompt，保留原请求和结果。仅改提示词不生成；明确要求重生成才用 regenerate_generation(taskKey)，沿用模型、输入与参数。request_generation 创建媒体任务，text 必须是完整模型提示词，程序不会追加脚本；references 明确素材 ID、用途与 role，省略时仅沿用本轮输入框引用，不自动补入工程素材。模型按用户选择，任务状态不等于生成完成或视觉通过。choose_take 选择素材，assemble_plan 编排镜头；时间线 start 为成片起点，trimIn/trimOut 为源区间，speed 为绝对倍速。失败按返回原因修正；状态不确定时先查目标，避免重复提交。创作方法与角色交接按适用 skill 执行。".into()
}

pub async fn execute_local(t: &ProjectTool, call: &ToolCall, prefix: &str) -> Value {
    let mut args = call.function.arguments.clone();
    if call.function.name == "mstudio_reopen_image" {
        if !profiles::allows(&t.profile, "inspect") {
            return json!({"error":"没有读取工程权限","code":"FORBIDDEN"});
        }
        let Some(id) = args["imageId"]
            .as_str()
            .filter(|id| id.starts_with("image-") && id.len() < 100)
        else {
            return json!({"error":"此工具只接受历史卸载提示中的 imageId。读取生成图片请用 mstudio_read_image(assetId)，素材 ID 从 inspect 获取，不要传文件名或任务 ID。","code":"INVALID_ARGS"});
        };
        let store = t.app.state::<crate::database::Store>();
        let raw=store.db.lock().unwrap().query_row(
            "SELECT json_extract(j.value,'$.image') FROM agent_events e,json_each(e.payload,'$.offloads') j WHERE e.project_id=?1 AND e.kind='image/offload' AND json_extract(j.value,'$.id')=?2 ORDER BY e.seq DESC LIMIT 1",
            rusqlite::params![t.project,id],|r|r.get::<_,String>(0),
        );
        return match raw {
            Ok(image) => match serde_json::from_str::<Value>(&image) {
                Ok(image) => json!({"ok":true,"imageId":id,"__offloadedImage":image}),
                Err(_) => json!({"error":"图片记录已损坏","code":"IMAGE_CORRUPT"}),
            },
            Err(rusqlite::Error::QueryReturnedNoRows) => {
                json!({"error":"找不到当前工程中的图片引用","code":"IMAGE_NOT_FOUND"})
            }
            Err(error) => json!({"error":error.to_string(),"code":"IMAGE_READ_FAILED"}),
        };
    }
    if call.function.name == "mstudio_memory" {
        return MemoryTool(t.clone())
            .call(&mut ToolContext::new(), args)
            .await
            .unwrap();
    }
    let Some(action) = call
        .function
        .name
        .strip_prefix("mstudio_")
        .filter(|a| ACTIONS.iter().any(|(name, _, _)| name == a))
    else {
        return json!({"error":"未知工具", "code":"UNKNOWN_TOOL"});
    };
    args["action"] = json!(action);
    t.execute(args, format!("{}:{}{}", t.turn, prefix, call.id.as_str()))
        .await
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_accepts_exact_task_lookup_but_not_edit_revision() {
        let schema = tool_schema::for_profile(&profiles::defaults(""));
        let fields = ACTIONS
            .iter()
            .find(|(name, _, _)| *name == "inspect")
            .unwrap()
            .2;
        let properties: serde_json::Map<_, _> = fields
            .iter()
            .map(|field| ((*field).to_owned(), schema["properties"][*field].clone()))
            .collect();
        let exposed = json!({"type":"object","properties":properties,"additionalProperties":false});
        assert!(
            super::super::schema::validate(
                &exposed,
                &json!({"section":"generation","taskKey":"retry:target"})
            )
            .is_ok()
        );
        assert!(
            super::super::schema::validate(
                &exposed,
                &json!({"section":"generation","taskKey":123})
            )
            .is_err()
        );
        assert!(
            super::super::schema::validate(
                &exposed,
                &json!({"nodeIds":["shot_01_hook"],"revision":680})
            )
            .is_err()
        );
    }
}
