use super::*;
use rig_core::message::ToolFunction;
#[test]
fn request_local_and_skill_reads_do_not_require_result_pagination() {
    let call = ToolCall::from_wire(
        "skill",
        rig_core::message::ToolFunction::new(
            "mstudio_read_skill".try_into().expect("nonempty tool name"),
            json!({"skill":"creative-ad-director"}),
        ),
    );
    let value = json!({"text":"camera method ".repeat(2000),"nextOffset":null});
    assert_eq!(project(&call, &value, Some("turn")), value);
    let editor = ToolCall::from_wire(
        "editor",
        ToolFunction::new(
            "str_replace_based_edit_tool".try_into().unwrap(),
            json!({"command":"view","path":"/scene.js"}),
        ),
    );
    assert_eq!(project(&editor, &value, None), value);
}
#[test]
fn model_results_hide_counters_but_keep_saved_content_and_conflicts() {
    let call = ToolCall::from_wire(
        "edit",
        ToolFunction::new("mstudio_edit".try_into().unwrap(), json!({})),
    );
    let value = json!({"applied":true,"revision":101,"storageVersion":205,
            "changes":[{"result":{"revision":102,"projectRevision":101,
                "savedValues":[{"id":"s1","values":{"text":"revision 101 is quoted user content"}}]}}],
            "conflicts":[{"target":"node:s2","code":"TARGET_CHANGED"}],
            "recovery":{"action":"inspect_targets"}});
    let view = project(&call, &value, Some("t"));
    assert!(view.get("revision").is_none());
    assert!(view.get("storageVersion").is_none());
    let saved = &view["changes"][0]["result"];
    assert!(saved.get("revision").is_none());
    assert!(saved.get("projectRevision").is_none());
    assert_eq!(
        saved["savedValues"],
        value["changes"][0]["result"]["savedValues"]
    );
    assert_eq!(view["conflicts"], value["conflicts"]);
    assert_eq!(view["recovery"], value["recovery"]);
    assert_eq!(value["revision"], 101);
    // Request-local tools may use their own revision-based editing protocol.
    assert_eq!(project(&call, &value, None), value);
}
#[test]
fn bounded_clip_page_is_delivered_without_a_second_model_read() {
    let call = ToolCall::from_wire(
        "clips",
        ToolFunction::new(
            "mstudio_inspect".try_into().expect("nonempty tool name"),
            json!({"section":"clips"}),
        ),
    );
    let value = json!({"items":[{"visual":"x".repeat(8000)}],"nextOffset":null});
    assert_eq!(project(&call, &value, Some("t")), value);
    let huge = json!({"items":"x".repeat(16000),"nextOffset":1});
    assert_eq!(project(&call, &huge, Some("t"))["detailOffloaded"], true);
    let delegate = ToolCall::from_wire(
        "child",
        ToolFunction::new(
            "mstudio_delegate".try_into().expect("nonempty tool name"),
            json!({}),
        ),
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
        ToolFunction::new(
            "mstudio_edit".try_into().expect("nonempty tool name"),
            json!({}),
        ),
    );
    let original = json!({"applied":true,"revision":12,"items":"中".repeat(12000)});
    let projected = project(&call, &original, Some("t"));
    assert!(projected.get("revision").is_none());
    assert!(!projected.to_string().contains("revision"));
    assert_eq!(original["revision"], 12);
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
    assert_eq!(
        serde_json::from_str::<Value>(&recovered).unwrap(),
        model_view(&original)
    );
    let db = store.db.lock().unwrap();
    let (seq, raw): (i64, String) = db.query_row(
            "SELECT seq,payload FROM agent_events WHERE project_id='p' AND turn_id='t' AND kind='tool/result'",
            [], |row| Ok((row.get(0)?, row.get(1)?)),
        ).unwrap();
    let canonical = crate::database::blobs::event(&db, seq, &raw).unwrap();
    assert_eq!(canonical["value"], original);
    drop(db);
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

#[test]
fn delegation_view_keeps_the_complete_conclusion_and_receipts() {
    use rig_core::message::ToolFunction;
    let call = ToolCall::from_wire(
        "delegate-1",
        ToolFunction::new(
            "mstudio_delegate".try_into().expect("nonempty tool name"),
            json!({}),
        ),
    );
    let value = json!({"ok":true,"answer":"设计理由".repeat(300),"changes":[{"result":{"applied":true,"revision":7,"savedValues":[{"id":"s4","exists":true,"complete":true,"values":{"shot":{"duration":1.5}}}]}}]});
    let view = project(&call, &value, Some("turn-1"));
    assert_eq!(view, model_view(&value));
    assert!(view["changes"][0]["result"].get("revision").is_none());
    assert_eq!(value["changes"][0]["result"]["revision"], 7);
    assert_eq!(value["answer"].as_str().unwrap().chars().count(), 1200);
}
