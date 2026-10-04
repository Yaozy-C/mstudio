use super::*;
pub(super) fn fixture() -> (Store, Value, AgentProfile) {
    let root = std::env::temp_dir().join(format!("mstudio-domain-{}", mstudio::media::id()));
    let store = Store::open(root).unwrap();
    let doc = json!({"id":"p","name":"Synthetic","revision":0,"brief":"","width":1280,"height":720,"fps":30,"assets":[],"nodes":[{"id":"a","kind":"note","title":"A","text":"old","x":0,"y":0,"width":280,"height":218},{"id":"b","kind":"note","title":"B","text":"other","x":0,"y":0,"width":280,"height":218}],"clips":[],"tracks":[{"id":"video","name":"Video","kind":"video"}],"captions":[]});
    crate::projects::write_document(&store, doc.clone(), true).unwrap();
    {
        let mut profile = crate::assistant::profiles::defaults("");
        profile.tool_ids = vec![
            "project-read".into(),
            "project-edit".into(),
            "project-production".into(),
            "media-generation".into(),
        ];
        let saved = load(&store.db.lock().unwrap(), "p").unwrap();
        (store, saved, profile)
    }
}
fn call(store: &Store, profile: &AgentProfile, id: &str, args: Value) -> Value {
    execute(store, profile, "p", "turn", id, args).unwrap()
}
#[test]
fn native_kernel_executes_without_ui_and_receipt_retry_is_idempotent() {
    let (store, _, profile) = fixture();
    call(
        &store,
        &profile,
        "read",
        json!({"action":"inspect","nodeIds":["a"]}),
    );
    let args = json!({"action":"edit","operations":[{"op":"update_node","id":"a","text":"new"}]});
    let result = call(&store, &profile, "write", args.clone());
    assert_eq!(result["outcome"], "committed", "{result}");
    assert_eq!(result, call(&store, &profile, "write", args));
    assert_eq!(load(&store.db.lock().unwrap(), "p").unwrap()["revision"], 1);
    std::fs::remove_dir_all(&store.root).unwrap();
}
#[test]
fn all_argument_errors_are_reported_before_any_effect() {
    let (store, doc, profile) = fixture();
    let result = call(
        &store,
        &profile,
        "bad",
        json!({"action":"edit","operations":[{"op":"update_node","id":"a","text":"must not save"},{"op":"request_generation","mediaKind":"video","duration":7,"text":null}]}),
    );
    assert_eq!(result["outcome"], "not_executed");
    let paths: Vec<_> = result["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|i| i["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"$.operations[1].duration"), "{result}");
    assert!(paths.contains(&"$.operations[1].text"), "{result}");
    assert_eq!(load(&store.db.lock().unwrap(), "p").unwrap(), doc);
    std::fs::remove_dir_all(&store.root).unwrap();
}
#[test]
fn unrelated_edits_merge_but_target_conflicts_return_a_narrow_read() {
    let (store, doc, profile) = fixture();
    call(
        &store,
        &profile,
        "read",
        json!({"action":"inspect","nodeIds":["a"]}),
    );
    let mut ui = doc.clone();
    ui["nodes"][1]["text"] = json!("UI changed B");
    crate::projects::save_merged(&store, ui, doc.clone()).unwrap();
    let edit =
        json!({"action":"edit","operations":[{"op":"update_node","id":"a","text":"agent A"}]});
    assert_eq!(
        call(&store, &profile, "write", edit.clone())["applied"],
        true
    );
    let mut local = doc.clone();
    local["nodes"][0]["x"] = json!(50);
    let merged = crate::projects::save_merged(&store, local, doc).unwrap();
    assert_eq!(merged["nodes"][0]["text"], "agent A");
    assert_eq!(merged["nodes"][1]["text"], "UI changed B");
    let mut next = merged.clone();
    next["nodes"][0]["text"] = json!("new human text");
    crate::projects::save_merged(&store, next, merged).unwrap();
    let failed = call(&store, &profile, "conflict", edit);
    assert_eq!(failed["conflicts"][0]["code"], "TARGET_CHANGED", "{failed}");
    assert_eq!(failed["conflicts"][0]["inspect"], json!({"nodeIds":["a"]}));
    std::fs::remove_dir_all(&store.root).unwrap();
}

#[test]
fn supplied_snapshot_observations_and_lost_tool_acknowledgement_survive_replay() {
    use crate::assistant::{
        context,
        harness::{context_source, session},
    };
    use rig_core::message::{AssistantContent, Message, ToolCall, ToolFunction};
    let (store, doc, profile) = fixture();
    let args = json!({"operations":[{"op":"update_node","id":"a","text":"saved once"}]});
    let call = ToolCall::from_wire(
        "write",
        ToolFunction {
            name: "mstudio_edit".into(),
            arguments: args.clone(),
        },
    );
    let messages = vec![
        context_source::snapshot(context::project_snapshot(&doc, Some("a"))),
        Message::Assistant {
            id: None,
            content: vec![AssistantContent::ToolCall(call)],
        },
    ];
    let binding = json!({"model":"synthetic"});
    session::start(&store, "p", "turn", binding.clone(), &messages).unwrap();
    // No inspect is needed: the user request already supplied this target state.
    let mut edit = args;
    edit["action"] = json!("edit");
    let receipt = execute(&store, &profile, "p", "turn", "turn:write", edit).unwrap();
    assert_eq!(receipt["applied"], true, "{receipt}");
    // Simulate a crash after transaction commit and before the journal result.
    let restored = session::restore(&store, "p", "turn", &binding)
        .unwrap()
        .unwrap();
    let last = serde_json::to_string(restored.last().unwrap()).unwrap();
    assert!(last.contains("committed"));
    assert!(!last.contains("EFFECT_UNKNOWN"));
    assert!(
        receipts::read(&store.db.lock().unwrap(), "another-project", "turn:write")
            .unwrap()
            .is_none()
    );
    std::fs::remove_dir_all(&store.root).unwrap();
}
