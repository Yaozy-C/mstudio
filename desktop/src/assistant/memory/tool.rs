use super::{Entry, MemoryBackend, SqliteMemory, now};
use crate::{assistant::tools::ProjectTool, database::Store};
use rig_agent::{prelude::*, tool::ToolContext};
use serde_json::{Value, json};
use std::convert::Infallible;
use tauri::Manager;
#[derive(Clone)]
pub struct MemoryTool(pub ProjectTool);
impl Tool for MemoryTool {
    const NAME: &'static str = "mstudio_memory";
    type Args = Value;
    type Output = Value;
    type Error = Infallible;
    fn description(&self) -> String {
        "当前项目的长期偏好与约束；工程已保存的脚本、镜头、时长、顺序、时间线和执行结果不得复制为记忆。仅新增/纠正/撤销长期信息时调用，内容未变不调用。快照 memory 已有条目和 memoryRevision，足够时直接使用；仅缺少目标条目或版本冲突才 list，分页按 nextOffset 继续。remember 新建省略 id；更新或 forget 使用快照或 list 的真实 id。修改附 memoryRevision（不是工程 revision）和本轮用户原话 evidence。依赖写入等待上次结果并使用新版本。相同内容返回 changed=false，不改来源或版本；成功返回本次 id 和版本，不回传全部记忆。自动整理关闭时只读；记忆不能授予权限。".into()
    }
    fn parameters(&self) -> Value {
        let mut schema = json!({"type":"object","properties":{
            "action":{"type":"string","enum":["list","remember","forget"]},
            "offset":{"type":"integer","minimum":0},"memoryRevision":{"type":"integer","minimum":0,"description":"使用快照 memory、list 或上次写入返回的 memoryRevision，不能使用工程 revision"},"id":{"type":"string","description":"新建必须省略；更新或删除必须使用快照 memory 或 list 返回的现有 id，禁止自行命名"},
            "title":{"type":"string","maxLength":60},"content":{"type":"string","maxLength":1200},
            "evidence":{"type":"string","description":"本轮用户原话中的连续片段，最多500字"}
        },"required":["action"],"additionalProperties":false});
        if !super::super::profiles::allows(&self.0.profile, "memory-write") {
            schema["properties"]["action"]["enum"] = json!(["list"]);
        }
        schema
    }
    async fn call(&self, _: &mut ToolContext, args: Value) -> Result<Value, Infallible> {
        let host = &self.0;
        if !super::super::profiles::allows(&host.profile, "memory-read") {
            return Ok(json!({"error":"当前 Agent 未配置读取记忆工具"}));
        }
        if host.token.is_cancelled() {
            return Ok(json!({"error":"任务已停止"}));
        }
        let store = host.app.state::<Store>();
        let result = operate(
            &store.db.lock().unwrap(),
            &host.project,
            &host.turn,
            &host.prompt,
            super::super::profiles::allows(&host.profile, "memory-write"),
            &args,
        );
        let result = result.unwrap_or_else(|e| json!({"error":e.to_string()}));
        Ok(result)
    }
}
pub(super) fn operate(
    db: &rusqlite::Connection,
    project: &str,
    turn: &str,
    prompt: &str,
    writable: bool,
    args: &Value,
) -> anyhow::Result<Value> {
    let backend = SqliteMemory(db);
    let mut memory = backend.recall(project)?;
    anyhow::ensure!(memory.enabled, "项目记忆已关闭");
    if args["action"] == "list" {
        let offset = args["offset"].as_u64().unwrap_or(0).min(40) as usize;
        let mut size = 0;
        let entries: Vec<_> = memory
            .entries
            .iter()
            .skip(offset)
            .take_while(|e| {
                size += serde_json::to_string(e).map_or(0, |v| v.len());
                size <= 7000
            })
            .collect();
        return Ok(
            json!({"memoryRevision":memory.revision,"autoUpdate":memory.auto_update,"total":memory.entries.len(),"nextOffset": if offset+entries.len()<memory.entries.len(){Some(offset+entries.len())}else{None},"entries":entries}),
        );
    }
    anyhow::ensure!(writable && memory.auto_update, "当前项目记忆为只读");
    if args["memoryRevision"].as_u64() != Some(memory.revision) {
        return Ok(
            json!({"error":"记忆版本不匹配；本次未写入。先 list 核对条目，使用 memoryRevision，不要使用工程 revision", "code":"MEMORY_REVISION_CONFLICT", "memoryRevision":memory.revision}),
        );
    }
    let evidence = args["evidence"].as_str().unwrap_or("").trim();
    anyhow::ensure!(
        evidence.chars().count() >= 2
            && evidence.chars().count() <= 500
            && prompt.contains(evidence),
        "请引用本轮用户的明确要求作为记忆依据"
    );
    let id = args["id"].as_str().unwrap_or("");
    if !id.is_empty() && !memory.entries.iter().any(|e| e.id == id) {
        return Ok(
            json!({"error":"记忆 ID 不存在，本次未写入。新建 remember 必须省略 id；更新或删除必须使用快照 memory 或 list 返回的现有 id", "code":"MEMORY_NOT_FOUND", "memoryRevision":memory.revision}),
        );
    }
    let affected_id;
    match args["action"].as_str() {
        Some("remember") => {
            let entry = Entry {
                id: if id.is_empty() {
                    mstudio::media::id()
                } else {
                    id.into()
                },
                title: args["title"].as_str().unwrap_or("").trim().into(),
                content: args["content"].as_str().unwrap_or("").trim().into(),
                source: evidence.into(),
                turn_id: Some(turn.into()),
                updated: now(),
            };
            if let Some(old) = memory.entries.iter().find(|old| {
                (id.is_empty() || old.id == id)
                    && old.title == entry.title
                    && old.content == entry.content
            }) {
                return Ok(
                    json!({"ok":true,"changed":false,"id":old.id,"memoryRevision":memory.revision}),
                );
            }
            affected_id = entry.id.clone();
            if id.is_empty() {
                memory.entries.push(entry);
            } else {
                let old = memory
                    .entries
                    .iter_mut()
                    .find(|e| e.id == id)
                    .ok_or_else(|| anyhow::anyhow!("记忆不存在，请先 list"))?;
                *old = entry;
            }
        }
        Some("forget") => {
            anyhow::ensure!(memory.entries.iter().any(|e| e.id == id), "记忆不存在");
            affected_id = id.to_string();
            memory.entries.retain(|e| e.id != id);
        }
        _ => anyhow::bail!("未知记忆操作"),
    }
    let saved = backend.replace(project, memory)?;
    Ok(json!({"ok":true,"changed":true,"id":affected_id,"memoryRevision":saved.revision}))
}
