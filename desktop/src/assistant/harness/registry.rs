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
        "读取当前任务所需工程内容；生成任务用 section=generation 与 taskKey 精确查询；批量任务按 turnId/status 筛选，先用 fields=[status,targetNodeId,resultAssetIds,error,trackingPaused] 获取每页最多30项的进度与整批 batch 统计，按 nextOffset 继续；镜头用 nodeIds/fields；脚本用 fields=[script]、paragraphIds 和 scriptFields 精确读取。省略 fields 只返回摘要。列表每页最多12项并受大小限制；8镜等小组可一次读取，优先指定必要 fields，按 nextOffset 补读。互不依赖的读取同批调用。revision 是返回值，不是查询参数；修改前核实目标最新状态。",
        &[
            "section",
            "ids",
            "taskKey",
            "turnId",
            "status",
            "nodeIds",
            "fields",
            "paragraphIds",
            "scriptFields",
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
        "分页检索项目历史对话；taskId 可限定当前或已知任务，完整原文按 textOffset 继续读取。",
        &["offset", "messageId", "textOffset", "taskId"],
    ),
    (
        "models",
        "分页查看已启用媒体模型；指定 mediaModelId 读取该模型专属提示词规则。",
        &["offset", "mediaModelId"],
    ),
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
        let app = t.app.clone();
        let project = t.project.clone();
        let profile = self.media_profile.clone();
        let args = call.function.arguments.clone();
        tokio::task::spawn_blocking(move || {
            super::image_read::read(
                &app.state::<crate::database::Store>(),
                &project,
                &profile,
                &args,
            )
        })
        .await
        .unwrap_or_else(|e| json!({"error":format!("读取媒体失败：{e}")}))
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
    fn result_turn(&self) -> Option<&str> {
        self.tool.as_ref().map(|t| t.turn.as_str())
    }
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
        definitions.push(ToolDefinition { name:"mstudio_read_result".into(), description:"分页读取已卸载的完整工具结果。callId 来自 resultRef；省略 turnId 为当前轮，读取历史结果时使用它的 turnId。offset 按返回 nextOffset 继续。".into(), parameters:json!({"type":"object","properties":{"callId":{"type":"string"},"turnId":{"type":"string"},"offset":{"type":"integer","minimum":0}},"required":["callId"],"additionalProperties":false}) });
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
                description: "读取真实图片或按 time 抽取视频帧，图片直接返回模型。assetId 使用 inspect 的真实素材 ID。视频 time 默认是源视频秒数；提供 clipId 则是片段内秒数，自动换算裁切/变速并应用当前调色，但不含转场、叠加轨和字幕。transition:true 配合后片段 clipId 时，time 改为转场开始后的秒数，返回实际转场合成（不含叠加轨、字幕或声音）。检查开头、中间、结尾及接缝两侧，不能把抽样称为看完整片。当前模型须支持图片输入。".into(),
                parameters: json!({"type":"object","properties":{"assetId":{"type":"string"},"clipId":{"type":"string"},"transition":{"type":"boolean"},"time":{"type":"number","minimum":0}},"required":["assetId"],"additionalProperties":false}),
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
            definition.description.push_str(&format!("可用角色与权限：{}。交付公共白底资产用 project-assets，script 用 project-script，shots 用 project-shots，image-prompts/images 用 project-frames，video-prompts/videos 用 project-production，timeline 用 project-timeline；生成还需 media-generation。",json!(roster)));

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
            || matches!(
                call.function.name.as_str(),
                "mstudio_reopen_image" | "mstudio_read_result"
            )
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
    "按当前角色权限批量修改工程；revision 使用 inspect 返回值，操作原子保存且可撤销。只修改本轮目标，已有对象沿用 ID；add_node 必填 id/kind/title，shot 关联 screenplayId；省略字段保留原值。update_node 更新卡片：screenplay.script 为脚本，镜头 text 为动作与摄影设计，shot.prompt 为视频草稿，shot.framePrompt/frames 为画格草稿与图片。修改已有生成任务用 update_generation(taskKey,text)，不顺带覆盖镜头草稿；统一更新任务 prompt，保留已有结果；已发出的请求不会被追写。仅改提示词不生成；明确要求重生成才用 regenerate_generation(taskKey)，沿用模型、输入与参数。request_generation 创建媒体任务，text 必须是完整模型提示词，程序不会追加脚本；references 明确素材 ID、用途与 role，省略时仅沿用本轮输入框引用，不自动补入工程素材。模型按用户选择，任务状态不等于生成完成或视觉通过。choose_take 选择素材，assemble_screenplay 编排镜头；时间线 start 为成片起点，trimIn/trimOut 为源区间，speed 为绝对倍速。savedClips 返回实际保存的时间线变化，complete=true 时可用于参数核对，不必立即再读同一批片段；complete=false 时按需 inspect。参数回执不能代替抽帧或播放验收。失败按返回原因修正；状态不确定时先查目标，避免重复提交。创作方法与角色交接按适用 skill 执行。".into()
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
        let db = store.db.lock().unwrap();
        return super::stored_image::read(&db, &t.project, id);
    }
    if call.function.name == "mstudio_read_result" {
        return super::tool_output::read(t, &args);
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
#[path = "registry_tests.rs"]
mod tests;
