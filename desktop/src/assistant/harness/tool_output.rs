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
            Some("clips" | "assets" | "tracks" | "captions")
        ))
        || (call.function.name == "mstudio_edit" && value.get("savedClips").is_some())
        || (call.function.name == "mstudio_delegate"
            && value["changes"].as_array().is_some_and(|changes| {
                changes
                    .iter()
                    .any(|change| change["result"].get("savedClips").is_some())
            })) {
        14_000
    } else {
        LIMIT
    };
    if value.get("__offloadedImage").is_some() {
        return json!({"ok":true,"imageId":value["imageId"]});
    }
    if call.function.name == "mstudio_delegate"
        && value["answer"]
            .as_str()
            .is_some_and(|s| s.chars().count() > 600)
        && value.to_string().chars().count() <= limit
    {
        let mut view = value.clone();
        view["answer"] = json!(
            value["answer"]
                .as_str()
                .unwrap()
                .chars()
                .take(600)
                .collect::<String>()
        );
        view["answerTruncated"] = json!(true);
        view["resultRef"] = json!(call.id);
        view["turnId"] = json!(turn);
        view["readWith"] = json!(
            "mstudio_read_result(callId=resultRef, turnId=turnId, offset=0) 读取完整说明；未显示内容可能包含设计理由或待办。"
        );
        return view;
    }
    if call.function.name == "mstudio_read_result" || value.to_string().chars().count() <= limit {
        return value.clone();
    }
    let mut result = json!({"detailOffloaded":true,"resultRef":call.id.as_str(),"turnId":turn,"readWith":"mstudio_read_result(callId=resultRef, offset=0); 当前任务内；跨任务时补充 turnId","availableFields":value.as_object().map(|v|v.keys().collect::<Vec<_>>())});
    // Never turn an applied edit or a real error into a synthetic failure.
    for key in [
        "ok",
        "stopReason",
        "agentId",
        "childId",
        "status",
        "applied",
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
                        .unwrap_or("工具执行失败；读取完整结果查看详情")
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
        let owner:Option<String>=store.db.lock().unwrap().query_row("SELECT json_extract(payload,'$.binding.agentId') FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind='session/start' ORDER BY seq LIMIT 1",rusqlite::params![t.project,turn],|r|r.get(0)).ok();
        if owner.as_deref() != Some(&t.profile.id) {
            return json!({"error":"不能读取其他角色的结果","code":"FORBIDDEN"});
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
    let raw:Result<String,_>=store.db.lock().unwrap().query_row("SELECT json_extract(payload,'$.value') FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind='tool/result' AND json_extract(payload,'$.callId')=?3 ORDER BY seq DESC LIMIT 1",rusqlite::params![project,turn,call],|r|r.get(0));
    match raw {
        Ok(raw) => {
            // Image bytes are read through the media tools, never as base64 text.
            let mut value: Value = match serde_json::from_str(&raw) {
                Ok(v) => v,
                Err(e) => return json!({"error":e.to_string()}),
            };
            if let Some(object) = value.as_object_mut() {
                object.remove("__offloadedImage");
            }
            let text = value.to_string();
            let total = text.chars().count();
            json!({"turnId":turn,"callId":call,"text":text.chars().skip(offset).take(LIMIT).collect::<String>(),"totalCharacters":total,"nextOffset":if offset.saturating_add(LIMIT)<total{Some(offset+LIMIT)}else{None}})
        }
        Err(_) => {
            json!({"error":"当前工程找不到该工具结果；使用原结果的 turnId 和 callId","code":"RESULT_NOT_FOUND"})
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig_core::message::ToolFunction;
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
        assert_eq!(view["answerTruncated"], true);
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
fn delegation_view_keeps_receipts_and_points_to_complete_design() {
    use rig_core::message::ToolFunction;
    let call = ToolCall::from_wire(
        "delegate-1",
        ToolFunction {
            name: "mstudio_delegate".into(),
            arguments: json!({}),
        },
    );
    let value = json!({"ok":true,"answer":"设计理由".repeat(300),"changes":[{"result":{"applied":true,"revision":7,"savedValues":[{"id":"s4","shot":{"duration":1.5}}]}}]});
    let view = project(&call, &value, Some("turn-1"));
    assert_eq!(view["changes"], value["changes"]);
    assert_eq!(view["answerTruncated"], true);
    assert_eq!(view["resultRef"], "delegate-1");
    assert_eq!(view["turnId"], "turn-1");
    assert_eq!(value["answer"].as_str().unwrap().chars().count(), 1200);
}
