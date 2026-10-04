//! Resolve a child execution to the immutable user request, independently of UI lifetime.
use crate::database::Store;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};

pub fn request(
    db: &Connection,
    project: &str,
    execution: &str,
) -> anyhow::Result<Option<(String, Value)>> {
    let mut turn = execution.to_owned();
    for _ in 0..8 {
        let raw: Option<String> = db.query_row(
            "SELECT json_extract(attribution,'$.request') FROM agent_messages WHERE project_id=?1 AND role='user' AND json_extract(attribution,'$.turnId')=?2",
            params![project, turn], |r| r.get(0),
        ).optional()?.flatten();
        if let Some(raw) = raw {
            return Ok(Some((turn, serde_json::from_str(&raw)?)));
        }
        let parent: Option<String> = db
            .query_row(
                "SELECT parent_turn FROM subagent_runs WHERE project_id=?1 AND last_turn=?2",
                params![project, turn],
                |r| r.get(0),
            )
            .optional()?;
        let Some(parent) = parent else {
            return Ok(None);
        };
        turn = parent;
    }
    anyhow::bail!("子任务的请求归属无效")
}

pub fn production(store: &Store, project: &str, execution: &str) -> anyhow::Result<Value> {
    let Some((turn, request)) = request(&store.db.lock().unwrap(), project, execution)? else {
        return Ok(Value::Null);
    };
    let context = &request["production"];
    if context.is_null() {
        return Ok(Value::Null);
    }
    anyhow::ensure!(context["projectId"] == project, "生成请求不属于当前项目");
    Ok(json!({"turnId":turn,"turn":context}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn child_and_continuation_resolve_durable_parent_without_crossing_projects() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE agent_messages(project_id TEXT,role TEXT,attribution TEXT); CREATE TABLE subagent_runs(project_id TEXT,last_turn TEXT,parent_turn TEXT);").unwrap();
        let meta = json!({"turnId":"root","request":{"production":{"projectId":"p","models":{"image":"selected"}}}});
        db.execute(
            "INSERT INTO agent_messages VALUES('p','user',?1)",
            [meta.to_string()],
        )
        .unwrap();
        db.execute_batch("INSERT INTO subagent_runs VALUES('p','child-1','root');")
            .unwrap();
        assert_eq!(request(&db, "p", "child-1").unwrap().unwrap().0, "root");
        db.execute_batch("UPDATE subagent_runs SET last_turn='child-2';")
            .unwrap();
        let (_, saved) = request(&db, "p", "child-2").unwrap().unwrap();
        assert_eq!(saved["production"]["models"]["image"], "selected");
        assert!(request(&db, "other", "child-2").unwrap().is_none());
    }
}

/// A successful model response does not erase a failed media operation.
pub fn outcome(store: &Store, project: &str, turn: &str) -> anyhow::Result<Value> {
    let db = store.db.lock().unwrap();
    let mut stmt = db.prepare("SELECT kind,payload,seq FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind IN ('tool/call','tool/result') ORDER BY seq")?;
    let rows = stmt
        .query_map(params![project, turn], |r| {
            let raw: String = r.get(1)?;
            let value = crate::database::blobs::event(&db, r.get(2)?, &raw)
                .map_err(|e| rusqlite::Error::ToSqlConversionFailure(e.into()))?;
            Ok((r.get::<_, String>(0)?, value.to_string()))
        })?
        .collect::<Result<Vec<_>, _>>()?;
    let mut result = summarize(rows)?;
    let keys: Vec<String> = result["generationTasks"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|task| task["id"].as_str().map(str::to_owned))
        .collect();
    if !keys.is_empty() {
        result["generationTasks"] = crate::project_service::generation_wait::snapshot(
            &db, project, &keys,
        )?["generationTasks"]
            .take();
    }
    Ok(result)
}

fn summarize(rows: Vec<(String, String)>) -> anyhow::Result<Value> {
    let mut calls = std::collections::HashMap::new();
    let mut failures = std::collections::BTreeMap::new();
    let mut tasks = std::collections::BTreeMap::new();
    for (kind, raw) in rows {
        let event: Value = serde_json::from_str(&raw)?;
        if kind == "tool/call" {
            let mut targets: Vec<String> = event["arguments"]["operations"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|op| {
                    op["op"] == "request_generation" || op["op"] == "regenerate_generation"
                })
                .map(|op| {
                    json!([
                        op["id"],
                        op["mediaKind"],
                        op["canvasTaskKey"],
                        op["taskKey"]
                    ])
                    .to_string()
                })
                .collect();
            let media_kind = match event["name"].as_str() {
                Some("mstudio_generate_image" | "mstudio_generate_reference_image") => {
                    Some(json!("image"))
                }
                Some("mstudio_generate_video") => Some(json!("video")),
                Some("mstudio_regenerate_generation") => Some(Value::Null),
                _ => None,
            };
            if let Some(media_kind) = media_kind {
                let args = &event["arguments"];
                targets.push(
                    json!([
                        args["id"],
                        media_kind,
                        args["canvasTaskKey"],
                        args["taskKey"]
                    ])
                    .to_string(),
                );
            }
            if !targets.is_empty() {
                calls.insert(event["callId"].as_str().unwrap_or("").to_owned(), targets);
            }
        }
        if kind == "tool/result"
            && let Some(targets) = calls.get(event["callId"].as_str().unwrap_or(""))
        {
            let result = event.get("value").unwrap_or(&event["result"]);
            for target in targets {
                if result["error"].is_null() {
                    failures.remove(target);
                } else {
                    failures.insert(target.clone(), result["error"].clone());
                }
            }
            if let Some(created) = result["generationTasks"].as_array() {
                for task in created {
                    if let Some(id) = task["id"].as_str() {
                        tasks.insert(id.to_owned(), task.clone());
                    }
                }
            }
        }
    }
    let failure = failures.into_values().next().unwrap_or(Value::Null);
    Ok(json!({"error":failure,"generationTasks":tasks.into_values().collect::<Vec<_>>()}))
}

#[cfg(test)]
mod outcome_tests {
    use super::*;
    #[test]
    fn standalone_generation_tool_failures_and_receipts_are_tracked() {
        let call = |id: &str| {
            ("tool/call".into(),json!({"callId":id,"name":"mstudio_generate_video","arguments":{"id":"s","prompt":"Video"}}).to_string())
        };
        let failed = (
            "tool/result".into(),
            json!({"callId":"a","result":{"error":"Failed"}}).to_string(),
        );
        assert_eq!(
            summarize(vec![call("a"), failed.clone()]).unwrap()["error"],
            "Failed"
        );
        let success = (
            "tool/result".into(),
            json!({"callId":"b","result":{"generationTasks":[{"id":"task"}]}}).to_string(),
        );
        let result = summarize(vec![call("a"), failed, call("b"), success]).unwrap();
        assert!(result["error"].is_null());
        assert_eq!(result["generationTasks"][0]["id"], "task");
    }
    #[test]
    fn a_different_success_does_not_hide_failure_but_same_target_retry_can_recover() {
        let call = |id: &str, target: &str| {
            ("tool/call".into(), json!({"callId":id,"arguments":{"operations":[{"op":"request_generation","id":target,"mediaKind":"image"}]}}).to_string())
        };
        let result = |id: &str, value: Value| {
            (
                "tool/result".into(),
                json!({"callId":id,"result":value}).to_string(),
            )
        };
        let mut rows = vec![
            call("a", "shot1"),
            result("a", json!({"error":"missing model"})),
            call("b", "shot2"),
            result("b", json!({"generationTasks":[{"id":"b"}]})),
        ];
        assert_eq!(summarize(rows.clone()).unwrap()["error"], "missing model");
        rows.extend([
            call("c", "shot1"),
            result("c", json!({"generationTasks":[{"id":"c"}]})),
        ]);
        let recovered = summarize(rows).unwrap();
        assert!(recovered["error"].is_null());
        assert_eq!(recovered["generationTasks"].as_array().unwrap().len(), 2);
    }
}
