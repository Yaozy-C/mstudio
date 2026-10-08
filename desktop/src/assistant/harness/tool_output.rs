//! Preserve canonical results in the journal and expose bounded, recoverable model views.
use crate::{assistant::tools::ProjectTool, database::Store};
use rig_core::message::ToolCall;
use serde_json::{Value, json};
use tauri::Manager;
const LIMIT: usize = 6000;

pub fn project(call: &ToolCall, value: &Value, turn: Option<&str>) -> Value {
    // Bounded editing pages/receipts must not immediately require another read-result call.
    let limit = if (call.function.name == "mstudio_inspect"
        && matches!(
            call.function.arguments["section"].as_str(),
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
    if matches!(
        call.function.name.as_str(),
        "mstudio_read_result" | "mstudio_read_skill"
    ) || value.to_string().chars().count() <= limit
    {
        return value.clone();
    }
    let mut result = json!({"detailOffloaded":true,"resultRef":call.id.as_str(),"turnId":turn,"readWith":"mstudio_read_result(callId=resultRef, offset=0) for this task; supply turnId for another task.","availableFields":value.as_object().map(|v|v.keys().collect::<Vec<_>>())});
    if let Some(answer) = value["answer"]
        .as_str()
        .filter(|s| s.chars().count() <= LIMIT)
    {
        result["answer"] = json!(answer);
    }
    // Task state is control information, not optional transcript detail.
    for field in ["generationTasks", "updatedTasks"] {
        if let Some(tasks) = value[field].as_array() {
            result[field] = json!(tasks.iter().take(30).map(|task| json!({
                "id":task["id"],"status":task["status"],"continuation":task["continuation"],
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
        "status",
        "applied",
        "stage",
        "outcome",
        "waitEnded",
        "executionId",
        "issues",
        "conflicts",
        "recovery",
        "revision",
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
            }
            let text = value.to_string();
            let total = text.chars().count();
            json!({"turnId":turn,"callId":call,"text":text.chars().skip(offset).take(LIMIT).collect::<String>(),"totalCharacters":total,"nextOffset":if offset.saturating_add(LIMIT)<total{Some(offset+LIMIT)}else{None}})
        }
        Err(_) => {
            json!({"error":"Tool result not found in this project; use its original turnId and callId","code":"RESULT_NOT_FOUND"})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig_core::message::ToolFunction;
    #[test]
    fn skill_body_is_delivered_without_a_second_result_pagination() {
        let call = ToolCall::from_wire(
            "skill",
            rig_core::message::ToolFunction {
                name: "mstudio_read_skill".into(),
                arguments: json!({"skill":"creative-ad-director"}),
            },
        );
        let value = json!({"text":"camera method ".repeat(2000),"nextOffset":null});
        assert_eq!(project(&call, &value, Some("turn")), value);
    }
    #[test]
    fn bounded_clip_page_is_delivered_without_a_second_model_read() {
        let call = ToolCall::from_wire(
            "clips",
            ToolFunction {
                name: "mstudio_inspect".into(),
                arguments: json!({"section":"clips"}),
            },
        );
        let value = json!({"items":[{"visual":"x".repeat(8000)}],"nextOffset":null});
        assert_eq!(project(&call, &value, Some("t")), value);
        let huge = json!({"items":"x".repeat(16000),"nextOffset":1});
        assert_eq!(project(&call, &huge, Some("t"))["detailOffloaded"], true);
        let delegate = ToolCall::from_wire(
            "child",
            ToolFunction {
                name: "mstudio_delegate".into(),
                arguments: json!({}),
            },
        );
        let receipt = json!({"items":[{"id":"clip","values":"x".repeat(7000)}],"complete":true});
        let delegated = json!({"answer":"a".repeat(900),"changes":[{"result":{"applied":true,"savedClips":receipt}}]});
        let view = project(&delegate, &delegated, Some("t"));
        assert_eq!(view["changes"][0]["result"]["savedClips"], receipt);
        assert_eq!(view["answer"], delegated["answer"]);
    }
    #[test]
    fn large_success_and_errors_keep_identity_and_can_be_read_back() {
        let root = std::env::temp_dir().join(format!("mstudio-result-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        store
            .db
            .lock()
            .unwrap()
            .execute("INSERT INTO projects VALUES('p','test','{}',0)", [])
            .unwrap();
        let call = ToolCall::from_wire(
            "c",
            ToolFunction {
                name: "mstudio_edit".into(),
                arguments: json!({}),
            },
        );
        let original = json!({"applied":true,"revision":12,"items":"中".repeat(12000)});
        let projected = project(&call, &original, Some("t"));
        assert_eq!(projected["revision"], 12);
        assert_eq!(projected["applied"], true);
        assert!(projected.to_string().len() < 10000);
        crate::assistant::journal::append(
            &store,
            "p",
            "t",
            "tool/result",
            json!({"callId":"c","value":original}),
        )
        .unwrap();
        let mut recovered = String::new();
        let mut offset = 0;
        loop {
            let page = read_page(&store, "p", "t", "c", offset);
            recovered.push_str(page["text"].as_str().unwrap());
            if let Some(next) = page["nextOffset"].as_u64() {
                offset = next as usize
            } else {
                break;
            }
        }
        assert_eq!(serde_json::from_str::<Value>(&recovered).unwrap(), original);
        assert_eq!(
            read_page(&store, "other", "t", "c", 0)["code"],
            "RESULT_NOT_FOUND"
        );
        let error = project(
            &call,
            &json!({"error":"conflict","code":"STALE","details":"x".repeat(10000)}),
            Some("t"),
        );
        assert_eq!(error["error"], "conflict");
        assert_eq!(error["code"], "STALE");
        let long_error = project(&call, &json!({"error":"失败".repeat(6000)}), Some("t"));
        assert!(long_error["error"].as_str().is_some());
        assert_eq!(long_error["errorTruncated"], true);
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[cfg(test)]
#[test]
fn delegation_view_keeps_the_complete_conclusion_and_receipts() {
    use rig_core::message::ToolFunction;
    let call = ToolCall::from_wire(
        "delegate-1",
        ToolFunction {
            name: "mstudio_delegate".into(),
            arguments: json!({}),
        },
    );
    let value = json!({"ok":true,"answer":"设计理由".repeat(300),"changes":[{"result":{"applied":true,"revision":7,"savedValues":[{"id":"s4","exists":true,"complete":true,"values":{"shot":{"duration":1.5}}}]}}]});
    let view = project(&call, &value, Some("turn-1"));
    assert_eq!(view["changes"], value["changes"]);
    assert_eq!(view, value);
    assert_eq!(value["answer"].as_str().unwrap().chars().count(), 1200);
}
