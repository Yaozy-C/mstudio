use super::*;
pub(crate) fn fixture() -> (Store, Value, AgentProfile) {
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
fn concept_board_saves_merges_and_validates_cards() {
    let (store, _, mut profile) = fixture();
    profile.tool_ids = vec!["project-read".into(), "project-concepts".into()];
    let card = json!({
        "id": "the-hearing",
        "title": "THE HEARING",
        "oneLine": "A bear cited for sandwich theft opens a full picnic at its hearing.",
        "truth": "Scarcity makes people call hospitality theft.",
        "brandIdea": "There was never a theft problem; there was a portions problem.",
        "participant": "A bear",
        "want": "To be understood as a provider, not a thief.",
        "ritual": "Wildlife committee hearing",
        "evidence": "25L capacity, utensils, cold, portions",
        "reframe": "The problem was the portions.",
        "risk": "A cute bear collapses the deadpan.",
        "cost": "High"
    });
    let saved = call(
        &store,
        &profile,
        "save-concepts-1",
        json!({"action":"edit","operations":[{"op":"save_concepts","concepts":[card]}]}),
    );
    assert_eq!(saved["outcome"], "committed", "{saved}");
    let doc = load(&store.db.lock().unwrap(), "p").unwrap();
    assert_eq!(doc["concepts"][0]["title"], "THE HEARING");
    assert_eq!(doc["concepts"][0]["status"], "rough");
    assert_eq!(doc["concepts"][0]["id"], "the-hearing");
    assert_eq!(
        doc["concepts"][0]["truth"],
        "Scarcity makes people call hospitality theft."
    );
    assert_eq!(
        doc["concepts"][0]["brandIdea"],
        "There was never a theft problem; there was a portions problem."
    );

    let read = call(
        &store,
        &profile,
        "read-concepts",
        json!({"action":"inspect","section":"concepts"}),
    );
    assert_eq!(read["items"][0]["reframe"], "The problem was the portions.");
    assert_eq!(
        read["items"][0]["truth"],
        "Scarcity makes people call hospitality theft."
    );

    let updated = call(
        &store,
        &profile,
        "save-concepts-2",
        json!({"action":"edit","operations":[{"op":"save_concepts","mode":"merge","concepts":[{"id":"the-hearing","status":"selected"}]}]}),
    );
    assert_eq!(updated["outcome"], "committed", "{updated}");
    let doc = load(&store.db.lock().unwrap(), "p").unwrap();
    assert_eq!(doc["concepts"][0]["status"], "selected");
    assert_eq!(doc["concepts"][0]["title"], "THE HEARING");

    let invalid = call(
        &store,
        &profile,
        "save-concepts-3",
        json!({"action":"edit","operations":[{"op":"save_concepts","concepts":[{"title":"No other fields"}]}]}),
    );
    assert_eq!(invalid["outcome"], "not_executed", "{invalid}");
    let mut second = doc["concepts"][0].clone();
    second["id"] = json!("keep-me");
    call(
        &store,
        &profile,
        "add-second-concept",
        json!({"action":"edit","operations":[{"op":"save_concepts","concepts":[second]}]}),
    );
    let stale = call(
        &store,
        &profile,
        "stale-delete",
        json!({"action":"edit","operations":[{"op":"remove_concepts","ids":["the-hearing","missing"]}]}),
    );
    assert_eq!(stale["outcome"], "not_executed");
    assert_eq!(
        load(&store.db.lock().unwrap(), "p").unwrap()["concepts"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    let removed = call(
        &store,
        &profile,
        "delete-one-concept",
        json!({"action":"edit","operations":[{"op":"remove_concepts","ids":["the-hearing"]}]}),
    );
    assert_eq!(removed["outcome"], "committed", "{removed}");
    let remaining = load(&store.db.lock().unwrap(), "p").unwrap();
    assert_eq!(remaining["concepts"].as_array().unwrap().len(), 1);
    assert_eq!(remaining["concepts"][0]["id"], "keep-me");
    assert_eq!(remaining["nodes"], doc["nodes"]);
    let args = json!({"action":"edit","operations":[{"op":"remove_concepts","ids":["keep-me"]}]});
    let empty = call(&store, &profile, "delete-last-concept", args.clone());
    assert_eq!(empty["outcome"], "committed", "{empty}");
    assert_eq!(call(&store, &profile, "delete-last-concept", args), empty);
    assert!(
        load(&store.db.lock().unwrap(), "p").unwrap()["concepts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
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
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unnamed_legacy_tasks_can_be_read_and_saved_without_ui_dependencies() {
    let (store, mut doc, profile) = fixture();
    let task = json!({"key":"legacy","kind":"image","mode":"single","inputs":[],
        "prompt":"Original prompt","modelId":"","status":"COMPLETED",
        "turnId":"old-turn","createdAt":1});
    doc["production"] = json!({"drafts":{"legacy":task}});
    crate::projects::write_document(&store, doc.clone(), false).unwrap();
    let read = call(
        &store,
        &profile,
        "read-legacy",
        json!({"action":"inspect","section":"generation"}),
    );
    assert_eq!(read["items"][0]["title"], "未命名", "{read}");
    let mut changed = doc.clone();
    changed["nodes"][0]["text"] = json!("Edited note");
    let saved = crate::projects::save_merged(&store, changed, doc).unwrap();
    assert_eq!(saved["nodes"][0]["text"], "Edited note");
    assert_eq!(saved["production"]["drafts"]["legacy"], task);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
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
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
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
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
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
        ToolFunction::new(
            "mstudio_update_note"
                .try_into()
                .expect("nonempty tool name"),
            json!({"id":"a","text":"saved once"}),
        ),
    );
    let messages = vec![
        context_source::snapshot(context::project_snapshot(&doc, Some("a"))),
        Message::Assistant(rig_core::message::AssistantMessage::new(vec![
            AssistantContent::ToolCall(call),
        ])),
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
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn asset_reader_defaults_to_reviewed_story_assets_and_legacy_classification_respects_user_choice() {
    let (store, mut doc, profile) = fixture();
    doc["assets"] = json!([
        {"id":"story","name":"Person","kind":"image","inLibrary":true},
        {"id":"material","name":"Character asset moodboard.png","kind":"image","inLibrary":true},
        {"id":"override","name":"Explicit material","kind":"image","inLibrary":true,"usage":"material"},
        {"id":"draft","name":"Unreviewed character","kind":"image","generated":true}
    ]);
    crate::projects::write_document(&store, doc, false).unwrap();
    store.db.lock().unwrap().execute("INSERT INTO jobs VALUES('asset-job','p',?1)",[json!({"asset":{"id":"story"},"assets":[{"id":"story"},{"id":"override"},{"id":"draft"}],"shot":{"canvasGeneration":{"task":{"generationPurpose":"asset"}}}}).to_string()]).unwrap();
    let read = |args: Value| call(&store, &profile, "read-usage", args);
    let default = read(json!({"action":"inspect","section":"assets"}));
    assert_eq!(default["items"].as_array().unwrap().len(), 1, "{default}");
    assert_eq!(default["items"][0]["id"], "story");
    assert_eq!(default["items"][0]["usage"], "asset");
    let ordinary = read(json!({"action":"inspect","section":"assets","usage":"material"}));
    assert_eq!(ordinary["items"].as_array().unwrap().len(), 3);
    let exact = read(json!({"action":"inspect","section":"assets","ids":["material","draft"]}));
    assert_eq!(exact["items"].as_array().unwrap().len(), 2);
    let mut changed = load(&store.db.lock().unwrap(), "p").unwrap();
    let original = changed.clone();
    changed["assets"][0]["usage"] = json!("material");
    crate::projects::write_document(&store, changed, false).unwrap();
    let saved = load(&store.db.lock().unwrap(), "p").unwrap();
    assert_eq!(saved["assets"][0]["usage"], "material");
    assert_eq!(
        saved["assets"][0]["imageVersionId"],
        original["assets"][0]["imageVersionId"]
    );
    assert_eq!(saved["nodes"], original["nodes"]);
    assert!(
        read(json!({"action":"inspect","section":"assets"}))["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        read(json!({"action":"inspect","section":"assets","usage":"all"}))["items"]
            .as_array()
            .unwrap()
            .len(),
        3
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
