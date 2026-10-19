use super::*;

#[test]
fn archive_pending_tasks_without_a_model_preserves_history_and_requires_observation() {
    let (store, mut doc, _) = tests::fixture();
    let profile = crate::assistant::profiles::builtins()
        .into_iter()
        .find(|p| p.id == "coordinator")
        .unwrap();
    let task = json!({"key":"old","kind":"video","mode":"multi","inputs":[],
        "prompt":"Original","parameters":{"duration":5},"modelId":"removed-model",
        "status":"AWAITING_CONFIRMATION","turnId":"old-turn","createdAt":1});
    doc["production"] = json!({"drafts":{"old":task}});
    crate::projects::write_document(&store, doc, false).unwrap();
    let args = json!({"action":"edit","operations":[{"op":"archive_generation","taskKey":"old","archived":true}]});
    let conflict = execute(&store, &profile, "p", "turn", "unobserved", args.clone()).unwrap();
    assert_eq!(conflict["code"], "TARGET_CONFLICT", "{conflict}");
    execute(
        &store,
        &profile,
        "p",
        "turn",
        "read",
        json!({"action":"inspect","section":"generation","taskKey":"old"}),
    )
    .unwrap();
    let saved = execute(&store, &profile, "p", "turn", "archive", args.clone()).unwrap();
    assert_eq!(saved["outcome"], "committed", "{saved}");
    assert_eq!(saved["updatedTasks"][0]["archived"], true);
    assert_eq!(
        saved["updatedTasks"][0]["continuation"]["state"],
        "finished"
    );
    assert!(saved["updatedTasks"][0].get("prompt").is_none());
    assert_eq!(
        saved,
        execute(&store, &profile, "p", "turn", "archive", args).unwrap()
    );
    let document = load(&store.conn().unwrap(), "p").unwrap();
    let mut expected = task;
    expected["hiddenFromList"] = json!(true);
    assert_eq!(document["production"]["drafts"]["old"], expected);
    let restored = execute(&store, &profile, "p", "turn", "restore", json!({"action":"edit","operations":[{"op":"archive_generation","taskKey":"old","archived":false}]})).unwrap();
    assert_eq!(restored["outcome"], "committed", "{restored}");
    assert_eq!(restored["updatedTasks"][0]["archived"], false);
    assert_eq!(
        restored["updatedTasks"][0]["continuation"]["state"],
        "waiting_user"
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn archive_rejects_an_active_task_atomically() {
    let (store, mut doc, profile) = tests::fixture();
    let task = |key: &str, status: &str| json!({"key":key,"kind":"video","mode":"multi","inputs":[],"prompt":"Original","modelId":"","status":status});
    doc["production"] = json!({"drafts":{"old":task("old", "AWAITING_CONFIRMATION"),"busy":task("busy", "IN_PROGRESS")}});
    crate::projects::write_document(&store, doc, false).unwrap();
    for key in ["old", "busy"] {
        execute(
            &store,
            &profile,
            "p",
            "turn",
            key,
            json!({"action":"inspect","section":"generation","taskKey":key}),
        )
        .unwrap();
    }
    let before = load(&store.conn().unwrap(), "p").unwrap();
    let rejected = execute(
        &store,
        &profile,
        "p",
        "turn",
        "archive",
        json!({"action":"edit","operations":[
            {"op":"archive_generation","taskKey":"old","archived":true},
            {"op":"archive_generation","taskKey":"busy","archived":true}
        ]}),
    )
    .unwrap();
    assert_eq!(rejected["outcome"], "not_executed", "{rejected}");
    assert_eq!(load(&store.conn().unwrap(), "p").unwrap(), before);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
