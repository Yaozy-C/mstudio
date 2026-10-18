//! Preserve canonical results in the journal and expose bounded, recoverable model views.
use crate::{assistant::tools::ProjectTool, database::Store};
use rig_core::message::ToolCall;
use serde_json::{Value, json};
use tauri::Manager;
const LIMIT: usize = 6000;

// Counters belong to host persistence and conflict detection, not content versions.
// Keep canonical journal values intact and filter before sizing or paginating so
// previews, delegated receipts and read-result pages cannot expose them again.
fn model_view(value: &Value) -> Value {
    let mut view = value.clone();
    fn strip(value: &mut Value) {
        match value {
            Value::Object(fields) => {
                fields.retain(|key, value| {
                    !value.is_number()
                        || !matches!(
                            key.as_str(),
                            "revision" | "projectRevision" | "storageVersion"
                        )
                });
                for value in fields.values_mut() {
                    strip(value);
                }
            }
            Value::Array(items) => items.iter_mut().for_each(strip),
            _ => {}
        }
    }
    strip(&mut view);
    view
}

pub fn project(call: &ToolCall, value: &Value, turn: Option<&str>) -> Value {
    // Request-local hosts have no journal from which to retrieve offloaded results.
    if turn.is_none() {
        return value.clone();
    }
    let view = model_view(value);
    let value = &view;
    // Bounded editing pages/receipts must not immediately require another read-result call.
    let limit = if super::read_tools::is_read(&call.function.name)
        || (call.function.name == "mstudio_inspect"
            && matches!(
                call.function
                    .arguments
                    .get("section")
                    .and_then(Value::as_str),
                Some("clips" | "assets" | "tracks" | "captions" | "generation")
            ))
        || call.function.name == "mstudio_await_generation"
        || (super::operation_tools::is_edit(&call.function.name)
            && value.get("savedClips").is_some())
        || call.function.name == "mstudio_delegate"
    {
        14_000
    } else {
        LIMIT
    };
    if value.get("__offloadedImage").is_some() {
        return json!({"ok":true,"imageId":value["imageId"]});
    }
    if value.get("__offloadedMedia").is_some() {
        return json!({"ok":true,"mediaId":value["mediaId"]});
    }
    if matches!(
        call.function.name.as_str(),
        "mstudio_read_result" | "mstudio_read_skill" | "mstudio_read_agent_result"
    ) || value.to_string().chars().count() <= limit
    {
        return value.clone();
    }
    let mut result = json!({"detailOffloaded":true,"resultRef":call.id.wire().as_ref(),"turnId":turn,"readWith":"mstudio_read_result(callId=resultRef, offset=0) for this task; supply turnId for another task.","availableFields":value.as_object().map(|v|v.keys().collect::<Vec<_>>())});
    if let Some(answer) = value["answer"]
        .as_str()
        .filter(|s| s.chars().count() <= LIMIT)
    {
        result["answer"] = json!(answer);
    }
    if let Some(delivery) = value.get("delivery") {
        result["delivery"] = delivery.clone();
    }
    if let Some(saved) = value.get("savedWork") {
        result["savedWork"] = saved.clone();
    }
    // Task state is control information, not optional transcript detail.
    for field in ["generationTasks", "updatedTasks"] {
        if let Some(tasks) = value[field].as_array() {
            result[field] = json!(tasks.iter().take(30).map(|task| json!({
                "id":task["id"],"kind":task["kind"],"status":task["status"],"continuation":task["continuation"],
                "targetId":task["targetId"],"resultAssetIds":task["resultAssetIds"],"error":task["error"]
            })).collect::<Vec<_>>());
            result[format!("{field}Complete")] = json!(tasks.len() <= 30);
        }
    }
    // Never turn an applied edit or a real error into a synthetic failure.
    for key in [
        "ok",
        "stopReason",
        "agentId",
        "childId",
        "outputRef",
        "status",
        "applied",
        "stage",
        "outcome",
        "waitEnded",
        "executionId",
        "issues",
        "conflicts",
        "recovery",
        "code",
        "error",
        "warnings",
        "changed",
        "savedValues",
        "savedClips",
        "operations",
        "nextOffset",
        "nextTextOffset",
        "total",
        "section",
    ] {
        if let Some(v) = value.get(key) {
            if v.to_string().chars().count() <= 800 {
                result[key] = v.clone()
            } else if key == "error" {
                result[key] = json!(
                    v.as_str()
                        .unwrap_or("Tool failed; read the full result for details.")
                        .chars()
                        .take(400)
                        .collect::<String>()
                );
                result["errorTruncated"] = json!(true);
            } else {
                result[format!("{key}Preview")] =
                    json!(v.to_string().chars().take(400).collect::<String>());
            }
        }
    }
    result["preview"] = json!(value.to_string().chars().take(1800).collect::<String>());
    result
}

pub fn read(t: &ProjectTool, args: &Value) -> Value {
    let store = t.app.state::<Store>();
    let turn = args["turnId"].as_str().unwrap_or(&t.turn);
    if turn != t.turn && !crate::assistant::profiles::allows(&t.profile, "inspect") {
        let owner =
            crate::database::session_checkpoint::owner(&store.db.lock().unwrap(), &t.project, turn)
                .ok()
                .flatten();
        if owner.as_deref() != Some(&t.profile.id) {
            return json!({"error":"Cannot read another role's results","code":"FORBIDDEN"});
        }
    }
    read_page(
        &store,
        &t.project,
        args["turnId"].as_str().unwrap_or(&t.turn),
        args["callId"].as_str().unwrap_or(""),
        args["offset"].as_u64().unwrap_or(0) as usize,
    )
}

pub fn read_page(store: &Store, project: &str, turn: &str, call: &str, offset: usize) -> Value {
    let raw = (|| -> anyhow::Result<Value> {
        let db = store.db.lock().unwrap();
        if let Some(receipt) =
            crate::project_service::receipts::read(&db, project, &format!("{turn}:{call}"))?
        {
            return Ok(receipt);
        }
        let (seq,raw): (i64,String) = db.query_row("SELECT seq,payload FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind='tool/result' AND json_extract(payload,'$.callId')=?3 ORDER BY seq DESC LIMIT 1", rusqlite::params![project,turn,call],|r|Ok((r.get(0)?,r.get(1)?)))?;
        Ok(crate::database::blobs::event(&db, seq, &raw)?["value"].take())
    })();
    match raw {
        Ok(mut value) => {
            // Image bytes are read through media tools, never as base64 text.
            if let Some(object) = value.as_object_mut() {
                object.remove("__offloadedImage");
                object.remove("__offloadedMedia");
            }
            let text = model_view(&value).to_string();
            let total = text.chars().count();
            json!({"turnId":turn,"callId":call,"text":text.chars().skip(offset).take(LIMIT).collect::<String>(),"totalCharacters":total,"nextOffset":if offset.saturating_add(LIMIT)<total{Some(offset+LIMIT)}else{None}})
        }
        Err(_) => {
            json!({"error":"Tool result not found in this project; use its original turnId and callId","code":"RESULT_NOT_FOUND"})
        }
    }
}

#[cfg(test)]
#[path = "tool_output_tests.rs"]
mod tests;
