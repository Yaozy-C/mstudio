//! A bounded, non-recursive specialist run. Child events never pollute parent replay.
use super::{Host, ProjectHost, registry, session::Session};
use crate::assistant::{
    config::Profile,
    context, journal,
    memory::{MemoryBackend, SqliteMemory},
    profiles, provider, skills,
};
use crate::database::Store;
use rig_core::{
    completion::ToolDefinition,
    message::{Message, ToolCall},
};
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Mutex, OnceLock},
};
use tauri::Manager;
use tokio_util::sync::CancellationToken;
pub struct Context {
    pub profile: Profile,
    pub key: String,
    pub model_id: Option<String>,
    pub fork_history: Vec<Message>,
    pub deadline: tokio::time::Instant,
}
pub fn definition() -> ToolDefinition {
    ToolDefinition { name:"mstudio_delegate".into(), description:"向专业 Agent 委派明确任务。task 必须自包含：目标、对象引用、必须保留项及预期结果。默认 spawn 使用独立会话；需要父会话已完成历史时才用 fork。返回结束原因、答复与真实操作结果；失败不代表已发生操作回滚。".into(), parameters:json!({"type":"object","properties":{"agentId":{"type":"string"},"task":{"type":"string","minLength":1,"maxLength":8000},"provider":{"type":"string","enum":["spawn","fork"]},"modelId":{"type":"string"},"mode":{"type":"string","enum":["oneShot","continuable"]},"runInBackground":{"type":"boolean"}},"required":["agentId","task"],"additionalProperties":false}) }
}

pub fn control_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {name:"mstudio_send_message".into(),description:"向直属可继续子 Agent 发送后续任务。运行中的子 Agent 在下一个模型步骤读取；空闲子 Agent 开始新一轮。只返回消息接收 ID。".into(),parameters:json!({"type":"object","properties":{"agentId":{"type":"string"},"message":{"type":"string","maxLength":8000}},"required":["agentId","message"],"additionalProperties":false})},
        ToolDefinition {name:"mstudio_interrupt_agent".into(),description:"停止直属子 Agent 的当前轮次；子 Agent 仍可继续接收后续消息。".into(),parameters:json!({"type":"object","properties":{"agentId":{"type":"string"}},"required":["agentId"],"additionalProperties":false})},
        ToolDefinition {name:"mstudio_list_agents".into(),description:"列出当前 Agent 的可继续子 Agent 及运行、空闲、可恢复状态。".into(),parameters:json!({"type":"object","properties":{"scope":{"type":"string","enum":["children","descendants"]}},"additionalProperties":false})},
    ]
}
struct ActiveRun {
    generation: String,
    token: CancellationToken,
}
static ACTIVE: OnceLock<Mutex<HashMap<String, ActiveRun>>> = OnceLock::new();
fn active() -> &'static Mutex<HashMap<String, ActiveRun>> {
    ACTIVE.get_or_init(|| Mutex::new(HashMap::new()))
}
fn remove_active(id: &str, generation: &str) {
    let mut active = active().lock().unwrap();
    if active
        .get(id)
        .is_some_and(|run| run.generation == generation)
    {
        active.remove(id);
    }
}
struct ChildHost {
    inner: ProjectHost,
    id: String,
    scripts: Mutex<Vec<Value>>,
    changes: Mutex<Vec<Value>>,
    terminal_reason: Mutex<Option<String>>,
}
impl Host for ChildHost {
    fn result_turn(&self) -> Option<&str> {
        self.inner.result_turn()
    }
    fn token(&self) -> &CancellationToken {
        &self.inner.token
    }
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.inner.definitions()
    }
    fn parallel_safe(&self, call: &ToolCall) -> bool {
        self.inner.parallel_safe(call)
    }
    fn injected(&self) -> Result<Vec<Message>, String> {
        let tool = self.inner.tool.as_ref().ok_or("子 Agent 缺少项目上下文")?;
        super::mailbox::take_inbox(
            &tool.app.state::<Store>(),
            &self.id,
            &tool.project,
            &tool.turn,
        )
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        let tool = self.inner.tool.as_ref().ok_or("子 Agent 缺少项目上下文")?;
        // Persist first; only authoritative committed outcomes count as applied.
        journal::append(
            &tool.app.state::<Store>(),
            &tool.project,
            &tool.turn,
            kind,
            value.clone(),
        )
        .map_err(|e| e.to_string())?;
        if kind == "model/stop" {
            *self.terminal_reason.lock().unwrap() = value["stopReason"].as_str().map(str::to_owned);
        }
        if kind == "tool/result" && value["value"]["applied"] == true {
            self.changes.lock().unwrap().push(
                json!({"callId":value["callId"],"name":value["name"],"result":value["value"]}),
            );
            if let Some(scripts) = value["value"]["scripts"].as_array() {
                self.scripts.lock().unwrap().extend(scripts.clone());
            }
        }
        Ok(())
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        if call.function.name == "mstudio_read_image" {
            return self.inner.read_image(call).await;
        }
        registry::execute_local(
            self.inner.tool.as_ref().unwrap(),
            call,
            &format!("{}:", self.id),
        )
        .await
    }
}
pub async fn execute(host: &ProjectHost, call: &ToolCall) -> Value {
    run(host, call).await.unwrap_or_else(
        |error| json!({"ok":false,"stopReason":"error","error":error,"code":"DELEGATION_FAILED"}),
    )
}

async fn run(host: &ProjectHost, call: &ToolCall) -> Result<Value, String> {
    let parent = host.tool.as_ref().ok_or("委派不可用")?;
    if !profiles::allows(&parent.profile, "delegate") {
        return Err("没有委派权限".into());
    }
    let route = host.delegation.as_ref().ok_or("未配置本轮模型")?;
    let args = &call.function.arguments;
    let id = args["agentId"].as_str().ok_or("缺少专业 Agent")?;
    let task = args["task"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or("缺少明确任务")?;
    let provider_name = args["provider"].as_str().unwrap_or("spawn");
    if !["spawn", "fork"].contains(&provider_name) {
        return Err("子 Agent 后端不存在".into());
    }
    let mode = args["mode"].as_str().unwrap_or("oneShot");
    if !["oneShot", "continuable"].contains(&mode) {
        return Err("子 Agent 模式无效".into());
    }
    let background = args["runInBackground"]
        .as_bool()
        .unwrap_or(mode == "continuable");
    if id == parent.profile.id || id == "coordinator" {
        return Err("不能委派给统筹或自己".into());
    }
    let store = parent.app.state::<Store>();
    let profile =
        profiles::resolve(&store.db.lock().unwrap(), Some(id)).map_err(|e| e.to_string())?;
    let profile = scoped(profile, &parent.profile.id)?;

    let mut tool = parent.clone();
    tool.profile = profile;
    tool.skill_setting = profiles::skill_setting(&tool.profile);
    let raw: String = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT document FROM projects WHERE id=?1",
            [&parent.project],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let doc: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let mut snapshot = if profiles::allows(&tool.profile, "inspect") {
        context::project_snapshot(&doc, None)
    } else {
        json!({})
    };
    if profiles::allows(&tool.profile, "memory-read") {
        snapshot["memory"] = crate::assistant::memory::context(
            &SqliteMemory(&store.db.lock().unwrap())
                .recall(&parent.project)
                .map_err(|e| e.to_string())?,
            &parent.prompt,
        );
    }
    snapshot["skills"] = skills::catalog(
        &skills::root(&parent.app).map_err(|e| e.to_string())?,
        &tool.skill_setting,
    )
    .map_err(|e| e.to_string())?;
    snapshot["agent"] = json!({"name":tool.profile.name,"instructions":tool.profile.instructions,"skills":tool.profile.skill_ids,"tools":tool.profile.tool_ids,"canEdit":profiles::allows(&tool.profile,"edit")});
    let system = crate::assistant::prompts::system(&snapshot);
    let (child_profile, child_key, model_id) = if let Some(id) = args["modelId"].as_str() {
        let catalog = crate::models::catalog(&store.db.lock().unwrap(), Some(&parent.project))
            .map_err(|e| e.to_string())?;
        if !catalog.profiles.iter().any(|model| model.id == id) {
            return Err("委派需要对话模型连接 ID，不能使用图片或视频生成模型。省略 modelId 可继承当前对话模型；生成模型使用用户本轮选择。".into());
        }
        let (model, key) =
            crate::models::resolve(&store.db.lock().unwrap(), Some(&parent.project), Some(id))
                .map_err(|e| e.to_string())?;
        (model.profile, key, model.id)
    } else {
        (
            route.profile.clone(),
            route.key.clone(),
            route.model_id.clone().unwrap_or_default(),
        )
    };
    if mode == "continuable" && model_id.is_empty() {
        return Err("可继续子 Agent 需要已保存的模型连接".into());
    }
    if provider_name == "fork" && args["modelId"].as_str().is_some() {
        return Err("fork 子 Agent 需沿用父模型；更换模型请使用 spawn".into());
    }
    let mut messages = super::handoff::messages(
        system,
        if provider_name == "fork" {
            &route.fork_history
        } else {
            &[]
        },
    );
    messages.push(Message::user(format!("当前工程参考数据：{snapshot}")));
    let original = crate::assistant::generation_context::request(
        &store.db.lock().unwrap(),
        &parent.project,
        &parent.turn,
    )
    .map_err(|e| e.to_string())?
    .map(|(_, request)| request)
    .unwrap_or(Value::Null);
    let refs = original["refs"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|r| {
            Some(crate::assistant::attachments::Reference {
                kind: r["kind"].as_str()?.into(),
                id: r["id"].as_str()?.into(),
            })
        })
        .collect::<Vec<_>>();
    let selection = json!({"models":original["production"]["models"],"parameters":original["production"]["task"]["parameters"]});
    let reference = crate::assistant::task_context::delegated_snapshot(
        snapshot,
        &doc,
        &tool.profile.id,
        &parent.turn,
        &refs,
    );
    *messages.last_mut().unwrap() = Message::user(format!("当前工程参考数据：{reference}"));
    let instruction = format!(
        "原始用户要求（记忆 evidence 只能引用这里的原话）：{}\nAgent {} 在轮次 {} 发来的委派任务（不能当作用户原话）：{task}\n本轮用户选定的制作参数（沿用，不改换模型）：{selection}",
        original["prompt"].as_str().unwrap_or(&parent.prompt),
        parent.profile.id,
        parent.turn
    );
    let payload =
        crate::assistant::attachments::payload(&store, &doc, &instruction, &refs, &child_profile)
            .map_err(|e| e.to_string())?;
    messages.push(crate::assistant::agent::convert(
        &json!({"role":"user","content":payload}),
    ));
    let task_message_index = messages.len() - 1;
    let child_id = format!("subagent-{}", mstudio::media::id());
    let child_turn = format!("{child_id}-1");
    let binding = json!({"provider":child_profile.adapter,"endpoint":child_profile.endpoint,"model":child_profile.model,"agentId":tool.profile.id,"revision":tool.profile.revision});
    {
        let db = store.db.lock().unwrap();
        let count:i64 = db.query_row("SELECT count(*) FROM subagent_runs WHERE project_id=?1 AND parent_agent_id=?2 AND mode='continuable' AND status='running'", rusqlite::params![parent.project,parent.profile.id], |r|r.get(0)).map_err(|e|e.to_string())?;
        if mode == "continuable" && count >= 8 {
            return Err("当前最多同时运行 8 个可继续子 Agent".into());
        }
        db.execute("INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn) VALUES(?1,?2,?3,?4,?5,?6,'running',?7,?8,?9)",rusqlite::params![child_id,parent.project,parent.turn,parent.profile.id,id,mode,serde_json::to_string(&tool.profile).map_err(|e|e.to_string())?,model_id,child_turn]).map_err(|e|e.to_string())?;
    }
    journal::append(&store, &parent.project, &child_turn, "subagent/created", json!({"childId":child_id,"parentTurn":parent.turn,"agentId":id,"provider":provider_name,"mode":mode})).map_err(|e|e.to_string())?;
    super::session::start(&store, &parent.project, &child_turn, binding, &messages)?;
    journal::append(&store, &parent.project, &child_turn, "subagent/context", json!({
        "childId":child_id,"parentTurn":parent.turn,"provider":provider_name,
        "inheritedMessageCount":if provider_name=="fork" { route.fork_history.len() } else { 0 },
        "taskMessageIndex":task_message_index,
        "source":super::mailbox::agent_source(&parent.profile.id, &parent.turn),
        "profile":tool.profile
    })).map_err(|e| e.to_string())?;
    tool.turn = child_turn.clone();
    // Keep the immutable user evidence; a delegated task is not a user quote.
    tool.prompt = original["prompt"].as_str().unwrap_or(&parent.prompt).into();
    let token = if background {
        CancellationToken::new()
    } else {
        host.token.clone()
    };
    tool.token = token.clone();
    let child = ChildHost {
        inner: ProjectHost {
            media_profile: child_profile.clone(),
            token: token.clone(),
            tool: Some(tool),
            delegation: None,
        },
        id: child_id.clone(),
        scripts: Mutex::new(vec![]),
        changes: Mutex::new(vec![]),
        terminal_reason: Mutex::new(None),
    };
    if background {
        active().lock().unwrap().insert(
            child_id.clone(),
            ActiveRun {
                generation: child_turn.clone(),
                token,
            },
        );
        let spawned_id = child_id.clone();
        let generation = child_turn.clone();
        tokio::spawn(async move {
            let answer = drive_child(
                &child,
                &child_profile,
                &child_key,
                messages,
                tokio::time::Instant::now() + std::time::Duration::from_secs(20 * 60),
            )
            .await;
            settle(&child, &spawned_id, &answer, false);
            remove_active(&spawned_id, &generation);
            wake_pending(&child, &spawned_id);
        });
        return Ok(
            json!({"ok":true,"agentId":id,"childId":child_id,"mode":mode,"status":"running"}),
        );
    }
    let answer = drive_child(&child, &child_profile, &child_key, messages, route.deadline).await;
    settle(&child, &child_id, &answer, true);
    let scripts = child.scripts.lock().unwrap().clone();
    let changes = child.changes.lock().unwrap().clone();
    let mut result = json!({"ok":answer.is_ok(),"agentId":id,"childId":child_id,"stopReason":stop_reason(&answer, &child),"applied":!changes.is_empty(),"scripts":scripts,"changes":changes});
    result["generationTasks"] =
        crate::assistant::generation_context::outcome(&store, &parent.project, &child_turn)
            .map_err(|e| e.to_string())?["generationTasks"]
            .clone();
    match answer {
        Ok(text) => {
            result["answer"] = json!(text.chars().take(2400).collect::<String>());
        }
        Err(error) => result["error"] = json!(error),
    }
    Ok(result)
}

async fn drive_child(
    child: &ChildHost,
    profile: &Profile,
    key: &str,
    messages: Vec<Message>,
    deadline: tokio::time::Instant,
) -> Result<String, String> {
    let model = provider::builder(profile, key)?
        .build()
        .model_handle()
        .clone();

    super::driver::run_until(
        &model,
        profile,
        child,
        Session::new(messages),
        false,
        key,
        deadline,
    )
    .await
}
fn stop_reason(answer: &Result<String, String>, child: &ChildHost) -> String {
    if child.token().is_cancelled() {
        "aborted".into()
    } else if answer.is_ok() {
        "completed".into()
    } else {
        child
            .terminal_reason
            .lock()
            .unwrap()
            .clone()
            .unwrap_or_else(|| "error".into())
    }
}

fn settle(child: &ChildHost, id: &str, answer: &Result<String, String>, notified: bool) {
    if let Some(tool) = &child.inner.tool {
        let store = tool.app.state::<Store>();
        let mode: String = store
            .db
            .lock()
            .unwrap()
            .query_row("SELECT mode FROM subagent_runs WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .unwrap_or_default();
        let status = if answer.is_ok() {
            if mode == "oneShot" {
                "completed"
            } else {
                "idle"
            }
        } else if mode == "continuable" && child.inner.token.is_cancelled() {
            "ready"
        } else {
            "failed"
        };
        let output = answer
            .as_ref()
            .map(String::as_str)
            .unwrap_or_else(|e| e.as_str());
        let update = (|| -> Result<(), String> {
            let mut db = store.db.lock().unwrap();
            let tx = db.transaction().map_err(|e| e.to_string())?;
            let changed = tx.execute("UPDATE subagent_runs SET status=?2,output=?3,notified=?4,updated=unixepoch() WHERE id=?1 AND last_turn=?5",rusqlite::params![id,status,output,notified as i32,tool.turn]).map_err(|e| e.to_string())?;
            if changed == 1 && !notified {
                tx.execute("INSERT INTO subagent_notices(child_id,project_id,parent_agent_id,status,output) SELECT id,project_id,parent_agent_id,?2,?3 FROM subagent_runs WHERE id=?1",rusqlite::params![id,status,output]).map_err(|e| e.to_string())?;
            }
            tx.commit().map_err(|e| e.to_string())
        })();
        if let Err(error) = update {
            eprintln!("subagent settlement failed: {error}");
        }
        let _ = journal::append(
            &store,
            &tool.project,
            &tool.turn,
            "subagent/settled",
            json!({"childId":id,"status":status,"stopReason":stop_reason(answer, child),"output":output}),
        );
    }
}

pub fn notices(tool: &crate::assistant::tools::ProjectTool) -> Result<Vec<Message>, String> {
    super::mailbox::notices(
        &tool.app.state::<Store>(),
        &tool.project,
        &tool.profile.id,
        &tool.turn,
    )
}

pub async fn execute_control(host: &ProjectHost, call: &ToolCall) -> Value {
    let result = match call.function.name.as_str() {
        "mstudio_list_agents" => list_agents(host),
        "mstudio_interrupt_agent" => interrupt_agent(host, &call.function.arguments),
        "mstudio_send_message" => send_message(host, &call.function.arguments),
        _ => Err("未知子 Agent 控制工具".into()),
    };
    result.unwrap_or_else(|error| json!({"error":error,"code":"SUBAGENT_CONTROL_FAILED"}))
}
fn authorized_child(
    host: &ProjectHost,
    id: &str,
) -> Result<(String, String, String, String), String> {
    let parent = host.tool.as_ref().ok_or("子 Agent 控制不可用")?;
    let store = parent.app.state::<Store>();
    store.db.lock().unwrap().query_row("SELECT status,profile,model_id,last_turn FROM subagent_runs WHERE id=?1 AND project_id=?2 AND parent_agent_id=?3 AND mode='continuable'",rusqlite::params![id,parent.project,parent.profile.id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).optional().map_err(|e|e.to_string())?.ok_or("找不到直属可继续子 Agent".into())
}
fn list_agents(host: &ProjectHost) -> Result<Value, String> {
    let parent = host.tool.as_ref().ok_or("子 Agent 控制不可用")?;
    let store = parent.app.state::<Store>();
    let db = store.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT id,agent_id,status,mode,updated FROM subagent_runs WHERE project_id=?1 AND parent_agent_id=?2 AND mode='continuable' ORDER BY created,id").map_err(|e|e.to_string())?;
    let children = stmt.query_map(rusqlite::params![parent.project,parent.profile.id],|r| Ok(json!({"id":r.get::<_,String>(0)?,"agentId":r.get::<_,String>(1)?,"status":r.get::<_,String>(2)?,"mode":r.get::<_,String>(3)?,"updated":r.get::<_,i64>(4)?}))).map_err(|e|e.to_string())?.collect::<Result<Vec<_>,_>>().map_err(|e|e.to_string())?;
    Ok(json!({"children":children}))
}
fn interrupt_agent(host: &ProjectHost, args: &Value) -> Result<Value, String> {
    let id = args["agentId"].as_str().ok_or("缺少子 Agent ID")?;
    authorized_child(host, id)?;
    if let Some(run) = active().lock().unwrap().get(id) {
        run.token.cancel();
    }
    Ok(json!({"ok":true,"agentId":id}))
}
fn send_message(host: &ProjectHost, args: &Value) -> Result<Value, String> {
    let id = args["agentId"].as_str().ok_or("缺少子 Agent ID")?;
    let message = args["message"]
        .as_str()
        .filter(|s| !s.trim().is_empty() && s.len() <= 8000)
        .ok_or("消息需为 1–8000 字节")?;
    let (status, profile_json, model_id, last_turn) = authorized_child(host, id)?;
    let parent = host.tool.as_ref().ok_or("子 Agent 控制不可用")?;
    let store = parent.app.state::<Store>();
    let message_id = {
        let db = store.db.lock().unwrap();
        db.execute(
            "INSERT INTO subagent_inbox(child_id,text,source) VALUES(?1,?2,?3)",
            rusqlite::params![
                id,
                message,
                super::mailbox::agent_source(&parent.profile.id, &parent.turn).to_string()
            ],
        )
        .map_err(|e| e.to_string())?;
        db.last_insert_rowid()
    };
    if status != "running" {
        start_continuation(parent, id, &profile_json, &model_id, &last_turn)?;
    }
    Ok(json!({"ok":true,"agentId":id,"messageId":message_id}))
}
fn wake_pending(child: &ChildHost, id: &str) {
    let Some(parent) = child.inner.tool.as_ref() else {
        return;
    };
    let store = parent.app.state::<Store>();
    let pending = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM subagent_inbox WHERE child_id=?1 AND consumed=0)",
            [id],
            |r| r.get::<_, bool>(0),
        )
        .unwrap_or(false);
    if !pending {
        return;
    }
    let state = store.db.lock().unwrap().query_row(
        "SELECT status,profile,model_id,last_turn FROM subagent_runs WHERE id=?1 AND mode='continuable'",
        [id], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?)),
    ).optional().ok().flatten();
    if let Some((status, profile, model, turn)) = state
        && status != "running"
        && let Err(error) = start_continuation(parent, id, &profile, &model, &turn)
    {
        eprintln!("subagent continuation failed: {error}");
    }
}
fn start_continuation(
    parent: &crate::assistant::tools::ProjectTool,
    id: &str,
    profile_json: &str,
    model_id: &str,
    last_turn: &str,
) -> Result<(), String> {
    let store = parent.app.state::<Store>();
    let saved: profiles::AgentProfile =
        serde_json::from_str(profile_json).map_err(|e| e.to_string())?;
    let owner: String = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT parent_agent_id FROM subagent_runs WHERE project_id=?1 AND id=?2",
            rusqlite::params![parent.project, id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let profile = scoped(
        profiles::resolve(&store.db.lock().unwrap(), Some(&saved.id)).map_err(|e| e.to_string())?,
        &owner,
    )?;
    let (route, key) = crate::models::resolve(
        &store.db.lock().unwrap(),
        Some(&parent.project),
        Some(model_id),
    )
    .map_err(|e| e.to_string())
    .map(|(model, key)| (model.profile, key))?;
    let mut binding: Value = {
        let db = store.db.lock().unwrap();
        let raw:String=db.query_row("SELECT payload FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind='session/start' ORDER BY seq LIMIT 1",rusqlite::params![parent.project,last_turn],|r|r.get(0)).map_err(|e|e.to_string())?;
        serde_json::from_str::<Value>(&raw).map_err(|e| e.to_string())?["binding"].clone()
    };
    if binding["provider"] != route.adapter
        || binding["endpoint"] != route.endpoint
        || binding["model"] != route.model
    {
        return Err("子 Agent 的原模型连接已变更，无法自动恢复".into());
    }
    let mut messages = super::session::restore(&store, &parent.project, last_turn, &binding)?
        .ok_or("子 Agent 会话无法恢复")?;
    let document: String = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT document FROM projects WHERE id=?1",
            [&parent.project],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let mut snapshot = if profiles::allows(&profile, "inspect") {
        context::project_snapshot(
            &serde_json::from_str::<Value>(&document).map_err(|e| e.to_string())?,
            None,
        )
    } else {
        json!({})
    };
    snapshot["skills"] = skills::catalog(
        &skills::root(&parent.app).map_err(|e| e.to_string())?,
        &profiles::skill_setting(&profile),
    )
    .map_err(|e| e.to_string())?;
    snapshot["agent"] = json!({"name":profile.name,"instructions":profile.instructions,"skills":profile.skill_ids,"tools":profile.tool_ids,"canEdit":profiles::allows(&profile,"edit")});
    // Preserve this child's exact transcript; only refresh its configured role at the turn boundary.
    messages = super::handoff::messages(crate::assistant::prompts::system(&snapshot), &messages);
    let previous_revision = binding["revision"].clone();
    binding["revision"] = json!(profile.revision);
    let snapshot = crate::assistant::task_context::reference_snapshot(snapshot);
    let current = Message::user(format!("当前工程参考数据：{snapshot}"));
    if messages
        .iter()
        .rev()
        .find(|message| {
            serde_json::to_string(message)
                .unwrap_or_default()
                .contains("当前工程参考数据：")
        })
        .is_none_or(|old| serde_json::to_value(old).ok() != serde_json::to_value(&current).ok())
    {
        messages.push(current);
    }
    let user_evidence = crate::assistant::generation_context::request(
        &store.db.lock().unwrap(),
        &parent.project,
        last_turn,
    )
    .map_err(|e| e.to_string())?
    .and_then(|(_, request)| request["prompt"].as_str().map(str::to_owned))
    .unwrap_or_default();
    messages.push(Message::user(format!("原始用户要求（记忆 evidence 只能引用这些原话）：{user_evidence}；后续委派消息不属于用户原话。")));
    let claimed = store.db.lock().unwrap().execute(
        "UPDATE subagent_runs SET status='running',updated=unixepoch() WHERE id=?1 AND last_turn=?2 AND status!='running'",
        rusqlite::params![id,last_turn],
    ).map_err(|e| e.to_string())?;
    if claimed == 0 {
        return Ok(());
    }
    let new_turn = format!("{id}-{}", mstudio::media::id());
    if let Err(error) =
        super::session::start(&store, &parent.project, &new_turn, binding, &messages)
    {
        let _ = store.db.lock().unwrap().execute(
            "UPDATE subagent_runs SET status='ready' WHERE id=?1 AND last_turn=?2",
            rusqlite::params![id, last_turn],
        );
        return Err(error);
    }
    journal::append(
        &store,
        &parent.project,
        &new_turn,
        "subagent/configuration",
        json!({
            "childId":id,"previousRevision":previous_revision,"revision":profile.revision,
            "profile":profile,"restoredFrom":last_turn
        }),
    )
    .map_err(|e| e.to_string())?;
    store.db.lock().unwrap().execute("UPDATE subagent_runs SET last_turn=?2,profile=?4,notified=0,updated=unixepoch() WHERE id=?1 AND last_turn=?3",rusqlite::params![id,new_turn,last_turn,serde_json::to_string(&profile).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
    let mut tool = parent.clone();
    tool.profile = profile;
    tool.skill_setting = profiles::skill_setting(&tool.profile);
    tool.turn = new_turn.clone();
    tool.prompt = user_evidence;
    let token = CancellationToken::new();
    tool.token = token.clone();
    active().lock().unwrap().insert(
        id.into(),
        ActiveRun {
            generation: new_turn.clone(),
            token: token.clone(),
        },
    );
    let child = ChildHost {
        inner: ProjectHost {
            media_profile: route.clone(),
            tool: Some(tool),
            token,
            delegation: None,
        },
        id: id.into(),
        scripts: Mutex::new(vec![]),
        changes: Mutex::new(vec![]),
        terminal_reason: Mutex::new(None),
    };
    let child_id = id.to_owned();
    let generation = new_turn.clone();
    tokio::spawn(async move {
        let answer = drive_child(
            &child,
            &route,
            &key,
            messages,
            tokio::time::Instant::now() + std::time::Duration::from_secs(20 * 60),
        )
        .await;
        settle(&child, &child_id, &answer, false);
        remove_active(&child_id, &generation);
        wake_pending(&child, &child_id);
    });
    Ok(())
}

fn scoped(
    mut profile: profiles::AgentProfile,
    parent: &str,
) -> Result<profiles::AgentProfile, String> {
    if !profile.enabled || profile.id == parent || profile.id == "coordinator" {
        return Err("此 Agent 不可委派".into());
    }
    profile
        .tool_ids
        .retain(|tool| !["agent-delegate", "memory-write", "memory-read"].contains(&tool.as_str()));
    Ok(profile)
}
#[cfg(test)]
#[path = "delegation_tests.rs"]
mod tests;
