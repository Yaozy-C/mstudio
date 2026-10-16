use super::*;
use serde_json::json;
fn shot() -> Value {
    json!({"op":"add_node","id":"shot","kind":"shot","title":"Shot","text":"Action","shot":{"screenplayId":"script","scriptId":"paragraph","order":1,"duration":3}})
}
fn script() -> Value {
    json!({"op":"add_node","id":"script","kind":"screenplay","title":"Script","text":"","screenplay":{"script":[{"id":"paragraph","title":"Scene","action":"Action","duration":3}]}})
}
fn call(store: &Store, profile: &AgentProfile, id: &str, args: Value) -> Value {
    execute(store, profile, "p", "turn", id, args).unwrap()
}
#[test]
fn changed_paragraph_rejects_entire_shot_batch_before_domain_validation() {
    let (store, _, profile) = tests::fixture();
    assert_eq!(
        call(
            &store,
            &profile,
            "create",
            json!({"action":"edit","operations":[script()]})
        )["applied"],
        true
    );
    call(
        &store,
        &profile,
        "read",
        json!({"action":"inspect","nodeIds":["script"],"fields":["screenplay"]}),
    );
    let before = load(&store.db.lock().unwrap(), "p").unwrap();
    let mut changed = before.clone();
    changed["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["id"] == "script")
        .unwrap()["screenplay"]["script"] = json!([]);
    crate::projects::save_merged(&store, changed, before).unwrap();
    let saved = load(&store.db.lock().unwrap(), "p").unwrap();
    let args = json!({"action":"edit","operations":[{"op":"add_node","id":"note","kind":"note","title":"Note","text":""},shot()]});
    let result = call(&store, &profile, "stale", args.clone());
    assert_eq!(result["code"], "TARGET_CONFLICT", "{result}");
    assert_eq!(result["conflicts"][0]["target"], "node:script");
    assert_eq!(result["conflicts"][0]["code"], "TARGET_CHANGED");
    assert_eq!(load(&store.db.lock().unwrap(), "p").unwrap(), saved);
    call(
        &store,
        &profile,
        "reread",
        json!({"action":"inspect","nodeIds":["script"]}),
    );
    let result = call(&store, &profile, "invalid", args);
    assert_eq!(result["code"], "PARAGRAPH_NOT_FOUND", "{result}");
    assert_eq!(
        result["context"],
        json!({"nodeId":"shot","screenplayId":"script","paragraphId":"paragraph"})
    );
    assert_eq!(load(&store.db.lock().unwrap(), "p").unwrap(), saved);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn a_new_screenplay_and_shot_can_be_created_in_one_batch() {
    let (store, _, profile) = tests::fixture();
    let result = call(
        &store,
        &profile,
        "create",
        json!({"action":"edit","operations":[script(),shot()]}),
    );
    assert_eq!(result["applied"], true, "{result}");
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn deleted_screenplay_is_stale_but_absence_can_be_observed() {
    let (store, _, profile) = tests::fixture();
    call(
        &store,
        &profile,
        "create",
        json!({"action":"edit","operations":[script()]}),
    );
    let before = load(&store.db.lock().unwrap(), "p").unwrap();
    let mut changed = before.clone();
    changed["nodes"]
        .as_array_mut()
        .unwrap()
        .retain(|n| n["id"] != "script");
    crate::projects::save_merged(&store, changed, before).unwrap();
    let result = call(
        &store,
        &profile,
        "deleted",
        json!({"action":"edit","operations":[shot()]}),
    );
    assert_eq!(result["conflicts"][0]["code"], "TARGET_CHANGED");
    call(
        &store,
        &profile,
        "read-absence",
        json!({"action":"inspect","nodeIds":["script"]}),
    );
    let result = call(
        &store,
        &profile,
        "missing",
        json!({"action":"edit","operations":[shot()]}),
    );
    assert_eq!(result["code"], "SCREENPLAY_NOT_FOUND", "{result}");
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn image_agent_local_shot_edits_do_not_require_the_unread_or_changed_screenplay() {
    let (store, _, creator) = tests::fixture();
    assert_eq!(
        call(
            &store,
            &creator,
            "create",
            json!({"action":"edit","operations":[script(),shot()]})
        )["applied"],
        true
    );
    let agent = crate::assistant::profiles::builtins()
        .into_iter()
        .find(|p| p.id == "image")
        .unwrap();
    let before = load(&store.db.lock().unwrap(), "p").unwrap();
    let mut changed = before.clone();
    let asset = json!({"id":"frame","kind":"image","name":"Frame","path":"","preview":"","duration":0,"width":1,"height":1,"hasAudio":false});
    changed["assets"] = json!([asset]);
    changed["production"] = json!({"drafts":{"task":{"key":"task","kind":"image","mode":"single","inputs":[],"prompt":"Original image","modelId":"","status":"AWAITING_CONFIRMATION","targetNodeId":"shot","turnId":"original","createdAt":1}}});
    crate::projects::save_merged(&store, changed, before).unwrap();
    execute(
        &store,
        &agent,
        "p",
        "image-child",
        "read-shot",
        json!({"action":"inspect","nodeIds":["shot"]}),
    )
    .unwrap();
    let before = load(&store.db.lock().unwrap(), "p").unwrap();
    let mut changed = before.clone();
    changed["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["id"] == "script")
        .unwrap()["screenplay"]["script"][0]["action"] = json!("Writer revised the scene");
    crate::projects::save_merged(&store, changed, before).unwrap();
    let result = execute(&store, &agent, "p", "image-child", "frames", json!({"action":"edit","operations":[{"op":"update_node","id":"shot","shot":{"frames":[{"assetId":"frame","title":"Frame"}]}}]})).unwrap();
    assert_eq!(result["applied"], true, "{result}");
    execute(
        &store,
        &agent,
        "p",
        "image-child",
        "read-task",
        json!({"action":"inspect","section":"generation","taskKey":"task"}),
    )
    .unwrap();
    let result = execute(&store, &agent, "p", "image-child", "image-prompt", json!({"action":"edit","operations":[{"op":"update_generation","taskKey":"task","text":"New image prompt"}]})).unwrap();
    assert_eq!(result["applied"], true, "{result}");
    assert_eq!(store.db.lock().unwrap().query_row("SELECT count(*) FROM project_observations WHERE turn_id='image-child' AND target='node:script'", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    let before = load(&store.db.lock().unwrap(), "p").unwrap();
    let mut changed = before.clone();
    changed["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["id"] == "shot")
        .unwrap()["shot"]["framePrompt"] = json!("Human prompt edit");
    crate::projects::save_merged(&store, changed, before).unwrap();
    let saved = load(&store.db.lock().unwrap(), "p").unwrap();
    let conflict = execute(&store, &agent, "p", "image-child", "stale", json!({"action":"edit","operations":[{"op":"update_node","id":"shot","shot":{"frames":[]}}]})).unwrap();
    assert_eq!(conflict["conflicts"][0]["target"], "node:shot");
    assert_eq!(conflict["conflicts"][0]["code"], "TARGET_CHANGED");
    assert_eq!(load(&store.db.lock().unwrap(), "p").unwrap(), saved);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn rebinding_a_shot_keeps_script_conflict_protection() {
    let (store, _, profile) = tests::fixture();
    assert_eq!(
        call(
            &store,
            &profile,
            "create",
            json!({"action":"edit","operations":[script(),shot()]})
        )["applied"],
        true
    );
    call(
        &store,
        &profile,
        "read",
        json!({"action":"inspect","nodeIds":["shot"]}),
    );
    let before = load(&store.db.lock().unwrap(), "p").unwrap();
    let mut changed = before.clone();
    let script = changed["nodes"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|n| n["id"] == "script")
        .unwrap();
    script["screenplay"]["script"]
        .as_array_mut()
        .unwrap()
        .push(json!({"id":"next","title":"Next","action":"New action","duration":3}));
    crate::projects::save_merged(&store, changed, before).unwrap();
    let saved = load(&store.db.lock().unwrap(), "p").unwrap();
    let result = call(
        &store,
        &profile,
        "rebind",
        json!({"action":"edit","operations":[{"op":"update_node","id":"shot","shot":{"scriptId":"next"}}]}),
    );
    assert_eq!(result["conflicts"][0]["target"], "node:script", "{result}");
    assert_eq!(result["conflicts"][0]["code"], "TARGET_CHANGED");
    assert_eq!(load(&store.db.lock().unwrap(), "p").unwrap(), saved);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
