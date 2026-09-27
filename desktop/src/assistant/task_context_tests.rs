use super::*;
use crate::assistant::harness::{session, session_selection};
use rig_core::message::Message;
fn scope(id: &str, agent: &str, targets: &[&str]) -> Scope {
    Scope {
        original_instruction: String::new(),
        task_id: id.into(),
        agent_id: agent.into(),
        targets: targets.iter().map(|s| s.to_string()).collect(),
        view: "film".into(),
    }
}
#[test]
fn follows_same_task_across_views_and_targets_but_isolates_roles() {
    let old = scope("a", "color", &["clip:1"]);
    assert_eq!(
        choose(
            Some(old.clone()),
            "color",
            "b",
            vec![],
            "film".into(),
            false
        ),
        old
    );
    let continued = choose(
        Some(old.clone()),
        "color",
        "b",
        vec!["clip:2".into()],
        "storyboard".into(),
        false,
    );
    assert_eq!(continued.task_id, "a");
    assert_eq!(continued.targets, vec!["clip:2"]);
    assert_eq!(continued.view, "storyboard");
    assert_eq!(
        choose(
            Some(old.clone()),
            "transition",
            "b",
            vec![],
            "film".into(),
            false
        )
        .task_id,
        "b"
    );
    assert_eq!(
        choose(Some(old), "color", "b", vec![], "film".into(), true).task_id,
        "b"
    );
}
#[test]
fn scoped_history_pairs_by_turn_and_restores_after_another_role() {
    let root = std::env::temp_dir().join(format!("mstudio-scope-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','test','{}',0)", [])
        .unwrap();
    for (turn, agent, task, status) in [
        ("a", "color", "task1", "completed"),
        ("b", "storyboard", "task2", "completed"),
        ("c", "color", "task3", "completed"),
        ("d", "color", "task1", "cancelled"),
    ] {
        let scope = scope(task, agent, &["clip:1"]);
        history::append_attributed(
            &store,
            "p",
            "request",
            &json!("request"),
            "answer",
            "m",
            Some(&json!({"turnId":turn,"agentId":agent,"taskScope":scope,"status":status})),
        )
        .unwrap();
        session::start(
            &store,
            "p",
            turn,
            json!({"agentId":agent,"taskId":task}),
            &[Message::user(format!("state-{turn}"))],
        )
        .unwrap();
    }
    let history = history(&store, "p", &scope("task1", "color", &[])).unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].attribution.as_ref().unwrap()["turnId"], "a");
    let restored = session_selection::latest(
        &store,
        "p",
        &json!({"agentId":"storyboard","taskId":"task2"}),
    )
    .unwrap()
    .unwrap();
    assert!(
        serde_json::to_string(&restored)
            .unwrap()
            .contains("state-b")
    );
    assert!(
        session_selection::latest(&store, "p", &json!({"agentId":"color","taskId":"new"}))
            .unwrap()
            .is_none()
    );
    journal::append(&store, "p", "reset", "session/reset", json!({})).unwrap();
    assert!(
        session_selection::latest(
            &store,
            "p",
            &json!({"agentId":"storyboard","taskId":"task2"})
        )
        .unwrap()
        .is_none()
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn snapshot_preserves_shared_constraints_without_other_object_details() {
    let doc = json!({"brief":"保留原声","nodes":[{"id":"a","title":"目标"},{"id":"b","title":"无关秘密"}]});
    let base = crate::assistant::context::project_snapshot(&doc, None);
    let result = snapshot(base, &scope("t", "custom-role", &["node:a"]), &doc);
    assert_eq!(result["requirements"], "保留原声");
    assert_eq!(result["nodeCount"], 2);
    assert!(!result.to_string().contains("无关秘密"));
}

#[test]
fn persisted_scope_survives_retry_and_reset_starts_fresh() {
    let root = std::env::temp_dir().join(format!("mstudio-scope-resolve-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','test','{}',0)", [])
        .unwrap();
    let mut request:Request=serde_json::from_value(json!({"projectId":"p","prompt":"添加转场","clientTurnId":"first","modelName":"test","messageContext":{"work":{"view":"film"}},"attachments":[{"kind":"clip","id":"a"},{"kind":"clip","id":"b"}]})).unwrap();
    let first = resolve(&store, &request, "transition").unwrap();
    history::append_attributed(&store,"p","add",&json!("add"),"done","m",Some(&json!({"agentId":"transition","turnId":"first","status":"completed","taskScope":first}))).unwrap();
    journal::append(&store, "p", "first", "turn/start", json!({})).unwrap();
    session::start(
        &store,
        "p",
        "first",
        json!({"agentId":"transition","taskId":"first"}),
        &[Message::user("original evidence")],
    )
    .unwrap();
    request.client_turn_id = "next".into();
    request.attachments = None;
    assert_eq!(resolve(&store, &request, "transition").unwrap(), first);
    assert_eq!(resolve(&store, &request, "color").unwrap().task_id, "next");
    request.new_task = true;
    assert_eq!(
        resolve(&store, &request, "transition").unwrap().task_id,
        "next"
    );
    request.resume_turn_id = Some("first".into());
    assert_eq!(resolve(&store, &request, "transition").unwrap(), first);
    let mut binding = json!({"agentId":"transition"});
    session_selection::bind_task(&store, "p", "next", Some("first"), &mut binding).unwrap();
    assert_eq!(binding["taskId"], "first");
    assert!(
        session::restore(&store, "p", "first", &binding)
            .unwrap()
            .is_some()
    );
    request.resume_turn_id = Some("missing-scope".into());
    assert!(resolve(&store, &request, "transition").is_err());
    assert!(
        session_selection::bind_task(&store, "p", "next", Some("missing-scope"), &mut binding)
            .is_err()
    );
    request.resume_turn_id = None;
    request.new_task = false;
    journal::append(&store, "p", "reset", "session/reset", json!({})).unwrap();
    assert_eq!(
        resolve(&store, &request, "transition").unwrap().task_id,
        "next"
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
