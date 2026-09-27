//! Task identity selects model context; the project chat remains an unabridged audit log.
use super::{chat::Request, history, journal};
use crate::database::Store;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct Scope {
    pub task_id: String,
    pub agent_id: String,
    pub targets: Vec<String>,
    pub view: String,
    pub original_instruction: String,
}

pub fn resolve(store: &Store, request: &Request, agent: &str) -> Result<Scope, String> {
    let db = store.db.lock().unwrap();
    let prior: Option<String> = if let Some(turn) = &request.resume_turn_id {
        db.query_row("SELECT json_extract(attribution,'$.taskScope') FROM agent_messages WHERE project_id=?1 AND role='user' AND json_extract(attribution,'$.turnId')=?2", params![request.project_id,turn], |r| r.get(0)).optional()
    } else {
        db.query_row("SELECT json_extract(attribution,'$.taskScope') FROM agent_messages WHERE project_id=?1 AND role='user' AND json_extract(attribution,'$.agentId')=?2 AND json_type(attribution,'$.taskScope')='object' AND EXISTS(SELECT 1 FROM agent_events e WHERE e.project_id=?1 AND e.turn_id=json_extract(agent_messages.attribution,'$.turnId') AND e.seq>COALESCE((SELECT MAX(seq) FROM agent_events WHERE project_id=?1 AND kind='session/reset'),0)) ORDER BY id DESC LIMIT 1", params![request.project_id,agent], |r| r.get(0)).optional()
    }.map_err(|e| e.to_string())?.flatten();
    let prior = prior
        .map(|s| serde_json::from_str::<Scope>(&s))
        .transpose()
        .map_err(|e| e.to_string())?;
    if request.resume_turn_id.is_some() {
        return prior.ok_or_else(|| "缺少任务上下文，请作为新任务发送".into());
    }
    let mut targets: Vec<String> = request
        .attachments
        .iter()
        .flatten()
        .map(|r| format!("{}:{}", r.kind, r.id))
        .collect();
    if targets.is_empty() {
        if let Some(id) = request
            .task_node_id
            .as_ref()
            .or(request.selected_node_id.as_ref())
        {
            targets.push(format!("node:{id}"));
        }
        if let Some(id) = request
            .selection
            .as_ref()
            .and_then(|v| v["clipId"].as_str())
        {
            targets.push(format!("clip:{id}"));
        }
    }
    targets.sort();
    targets.dedup();
    let view = request.message_context["work"]["view"]
        .as_str()
        .unwrap_or("project")
        .to_owned();
    let mut scope = choose(
        prior,
        agent,
        &request.client_turn_id,
        targets,
        view,
        request.new_task,
    );
    if scope.original_instruction.is_empty() {
        scope.original_instruction = request.message_context["production"]["instruction"]
            .as_str()
            .unwrap_or(&request.prompt)
            .to_owned();
    }
    Ok(scope)
}

fn choose(
    prior: Option<Scope>,
    agent: &str,
    turn: &str,
    targets: Vec<String>,
    view: String,
    new_task: bool,
) -> Scope {
    if !new_task
        && let Some(mut prior) = prior
        && prior.agent_id == agent
    {
        prior.view = view;
        if !targets.is_empty() {
            prior.targets = targets;
        }
        return prior;
    }
    Scope {
        original_instruction: String::new(),
        task_id: turn.into(),
        agent_id: agent.into(),
        targets,
        view,
    }
}

pub fn saved(store: &Store, project: &str, turn: &str) -> Result<Option<Scope>, String> {
    let raw: Option<String> = store.db.lock().unwrap().query_row("SELECT json_extract(attribution,'$.taskScope') FROM agent_messages WHERE project_id=?1 AND role='user' AND json_extract(attribution,'$.turnId')=?2",params![project,turn],|r|r.get(0)).optional().map_err(|e|e.to_string())?.flatten();
    raw.map(|s| serde_json::from_str(&s).map_err(|e| e.to_string()))
        .transpose()
}

pub fn history(
    store: &Store,
    project: &str,
    scope: &Scope,
) -> Result<Vec<history::Message>, String> {
    // Read only six completed turns from this task, paired by turn ID rather than row parity.
    let rows = {
        let db = store.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT u.id,u.content,u.model,u.payload,u.attribution,a.id,a.content,a.model,a.payload,a.attribution FROM agent_messages u JOIN agent_messages a ON a.project_id=u.project_id AND a.role='assistant' AND json_extract(a.attribution,'$.turnId')=json_extract(u.attribution,'$.turnId') WHERE u.project_id=?1 AND u.role='user' AND json_extract(u.attribution,'$.taskScope.taskId')=?2 AND json_extract(u.attribution,'$.agentId')=?3 AND json_extract(a.attribution,'$.status')='completed' ORDER BY u.id DESC LIMIT 6").map_err(|e|e.to_string())?;
        stmt.query_map(params![project, scope.task_id, scope.agent_id], |r| {
            let make = |offset, role: &str| -> rusqlite::Result<history::Message> {
                let payload: String = r.get(offset + 3)?;
                let meta: String = r.get(offset + 4)?;
                Ok(history::Message {
                    id: r.get(offset)?,
                    role: role.into(),
                    content: r.get(offset + 1)?,
                    model: r.get(offset + 2)?,
                    payload: serde_json::from_str(&payload).unwrap_or(Value::Null),
                    attribution: serde_json::from_str(&meta).ok(),
                })
            };
            Ok([make(0, "user")?, make(5, "assistant")?])
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };
    Ok(rows.into_iter().rev().flatten().collect())
}

pub fn snapshot(mut snapshot: Value, scope: &Scope, doc: &Value) -> Value {
    let ids: Vec<_> = scope
        .targets
        .iter()
        .filter_map(|s| s.strip_prefix("node:"))
        .collect();
    if !scope.targets.is_empty() {
        snapshot["nodes"] = json!(
            snapshot["nodes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|n| ids.contains(&n["id"].as_str().unwrap_or("")))
                .collect::<Vec<_>>()
        );
        snapshot["relevantNodes"] = json!(ids.iter().filter_map(|id|doc["nodes"].as_array()?.iter().find(|n|n["id"]==*id)).map(|n|json!({"id":n["id"],"kind":n["kind"],"title":n["title"],"details":"按 nodeIds/fields inspect 读取"})).collect::<Vec<_>>());
        snapshot["tracks"] = json!([]);
    }
    snapshot["task"] = json!(scope);
    snapshot["contextPolicy"] = json!({"history":"仅当前角色和任务；其他历史可通过 history 按需读取","details":"省略不代表不存在；inspect 支持 ids、nodeIds、fields 与分页。修改前读取目标最新状态。","sharedConstraints":"requirements/creation 是有长度限制的项目约束预览；truncated 为 true 时先 inspect section=creation 补读完整约束。"});
    snapshot["constraintsTruncated"] = json!(
        doc["brief"].as_str().unwrap_or("").chars().count() > 2000
            || ["intent", "essential", "preserve"]
                .iter()
                .any(
                    |k| doc["creation"][k].as_str().unwrap_or("").chars().count()
                        > if *k == "intent" { 2000 } else { 1500 }
                )
    );
    snapshot
}

pub fn recovery(
    store: &Store,
    project: &str,
    scope: &Scope,
    resume: &str,
) -> Result<Value, String> {
    let turn:Option<String>=store.db.lock().unwrap().query_row("SELECT json_extract(attribution,'$.turnId') FROM agent_messages WHERE project_id=?1 AND role='assistant' AND json_extract(attribution,'$.taskScope.taskId')=?2 AND json_extract(attribution,'$.agentId')=?3 AND json_extract(attribution,'$.status') IN ('failed','cancelled','interrupted') AND json_extract(attribution,'$.turnId')=?4 ORDER BY id DESC LIMIT 1",params![project,scope.task_id,scope.agent_id,resume],|r|r.get(0)).optional().map_err(|e|e.to_string())?;
    let Some(turn) = turn else {
        return Ok(Value::Null);
    };
    let events = journal::turn_page(store, project, &turn, None).map_err(|e| e.to_string())?;
    let mut selected = Vec::new();
    for event in events.iter().rev().filter(|e| e["kind"] == "tool/result") {
        let payload = &event["payload"];
        let result = &payload["result"];
        let mut receipt = json!({"callId":payload["callId"],"name":payload["name"],"applied":result["applied"],"revision":result["revision"],"code":result["code"],"resultRef":{"turnId":turn,"callId":payload["callId"]}});
        for key in ["changed", "savedValues", "error"] {
            if !result[key].is_null() && result[key].to_string().chars().count() <= 600 {
                receipt[key] = result[key].clone();
            }
        }
        if selected.len() < 12 {
            selected.push(receipt);
        }
    }
    Ok(
        json!({"turnId":turn,"receipts":selected,"partial":true,"note":"仅部分写入回执；完整结果通过 mstudio_read_result 的 turnId/callId 读取。没有回执不代表未执行，未知状态先 inspect 核实，不自动重放。"}),
    )
}

#[cfg(test)]
#[path = "task_context_tests.rs"]
mod tests;

/// Child snapshots omit role/skill rules already installed as the system message.
pub fn delegated_snapshot(
    snapshot: Value,
    doc: &Value,
    role: &str,
    task: &str,
    refs: &[super::attachments::Reference],
) -> Value {
    let scope = Scope {
        original_instruction: String::new(),
        task_id: task.into(),
        agent_id: role.into(),
        targets: refs
            .iter()
            .map(|r| format!("{}:{}", r.kind, r.id))
            .collect(),
        view: "delegated".into(),
    };
    reference_snapshot(self::snapshot(snapshot, &scope, doc))
}
pub fn reference_snapshot(mut snapshot: Value) -> Value {
    if let Some(fields) = snapshot.as_object_mut() {
        fields.remove("agent");
        fields.remove("skills");
        fields.remove("specialists");
    }
    snapshot
}
