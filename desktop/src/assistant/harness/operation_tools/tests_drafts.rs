use super::tests::{decode, profile};
use super::*;

#[test]
fn direct_prompt_tool_commits_once_and_recovers_its_receipt() {
    use rig_core::message::{AssistantContent, Message, ToolCall, ToolFunction};
    let root = std::env::temp_dir().join(format!("mstudio-split-{}", mstudio::media::id()));
    let store = crate::database::Store::open(root.clone()).unwrap();
    let doc = json!({"id":"p","name":"Synthetic","revision":0,"brief":"","width":1280,"height":720,"fps":30,"assets":[],"nodes":[
        {"id":"script","kind":"screenplay","title":"Script","x":0,"y":0,"width":280,"height":218,"screenplay":{"script":[{"id":"paragraph","title":"Scene","action":"Action","duration":5}]}},
        {"id":"shot","kind":"shot","title":"Shot","text":"Preserved staging","x":320,"y":0,"width":280,"height":218,"shot":{"screenplayId":"script","scriptId":"paragraph","order":1,"duration":5,"dialogue":"Preserved dialogue","prompt":"Old"}}
    ],"clips":[],"tracks":[],"captions":[]});
    crate::projects::write_document(&store, doc, true).unwrap();
    let p = profile("production");
    crate::project_service::execute(
        &store,
        &p,
        "p",
        "turn",
        "read",
        json!({"action":"inspect","nodeIds":["shot","script"]}),
    )
    .unwrap();
    crate::assistant::history::append_attributed(&store, "p", "Save draft", &json!("Save draft"), "", "m", Some(&json!({"turnId":"turn","request":{"production":{"projectId":"p","models":{"execution":"automatic"},"instruction":"Save draft"}}}))).unwrap();
    let input = json!({"id":"shot","prompt":"New video prompt"});
    let args = decode(&p, "mstudio_save_video_generation", input.clone())
        .unwrap()
        .unwrap();
    let receipt =
        crate::project_service::execute(&store, &p, "p", "turn", "turn:call", args.clone())
            .unwrap();
    assert_eq!(receipt["outcome"], "committed", "{receipt}");
    assert_eq!(
        crate::project_service::execute(&store, &p, "p", "turn", "turn:call", args).unwrap(),
        receipt
    );
    let saved = crate::project_service::execute(
        &store,
        &p,
        "p",
        "turn",
        "verify",
        json!({"action":"inspect","nodeIds":["shot"],"fields":["prompt","dialogue","text"]}),
    )
    .unwrap()
    .to_string();
    assert!(!saved.contains("New video prompt"), "{saved}");
    let document: Value = serde_json::from_str(
        &store
            .db
            .lock()
            .unwrap()
            .query_row("SELECT document FROM projects WHERE id='p'", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
    )
    .unwrap();
    let draft = document["production"]["drafts"]
        .as_object()
        .unwrap()
        .values()
        .next()
        .unwrap();
    assert_eq!(draft["prompt"], "New video prompt");
    assert_eq!(draft["status"], "DRAFT");
    for value in ["Old", "Preserved dialogue", "Preserved staging"] {
        assert!(saved.contains(value), "{saved}");
    }
    let call = ToolCall::from_wire(
        "call",
        ToolFunction::new(
            "mstudio_save_video_generation"
                .try_into()
                .expect("nonempty tool name"),
            input,
        ),
    );
    let mut messages = vec![Message::Assistant(
        rig_core::message::AssistantMessage::new(vec![AssistantContent::ToolCall(call)]),
    )];
    super::super::session_recovery::repair(&store, "p", "turn", &mut messages);
    assert_eq!(messages.len(), 2);
    let restored = serde_json::to_string(&messages[1]).unwrap();
    assert!(restored.contains("committed"), "{restored}");
    assert!(!restored.contains("EFFECT_UNKNOWN"));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
