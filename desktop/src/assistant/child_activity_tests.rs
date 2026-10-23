use super::*;
#[test]
fn child_activity_is_scoped_to_parent_call_and_survives_terminal_states_and_restart() {
    let root =
        std::env::temp_dir().join(format!("mstudio-child-activity-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    {
        let db = store.db.lock().unwrap();
        db.execute("INSERT INTO projects VALUES('p','p','{}',0)", [])
            .unwrap();
        db.execute("INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id) VALUES('child','p','parent','coordinator','art','continuable','running','{}','model')",[]).unwrap();
        start(
            &db,
            "p",
            "child-1",
            "child",
            "parent",
            "delegate-call",
            "art",
        )
        .unwrap();
        assert!(
            update(
                &db,
                "p",
                "child-1",
                "session/message",
                &json!({"text":"private context"})
            )
            .unwrap()
            .is_none()
        );
        let value = update(
            &db,
            "p",
            "child-1",
            "tool/call",
            &json!({"name":"mstudio_read_assets","arguments":{"ids":["private-reference"]}}),
        )
        .unwrap()
        .unwrap();
        assert_eq!(value["phase"]["payload"]["name"], "mstudio_read_assets");
        assert!(!value.to_string().contains("private"));
        update(
            &db,
            "p",
            "child-1",
            "assistant/partial",
            &json!({"delta":"正在检查光线"}),
        )
        .unwrap();
        update(&db, "p", "child-1", "request/start", &json!({})).unwrap();
    }
    super::super::journal::append(
        &store,
        "p",
        "child-1",
        "tool/call",
        json!({"callId":"read","name":"mstudio_read_assets"}),
    )
    .unwrap();
    let children = list(&store, "p", "parent").unwrap();
    assert_eq!(children.len(), 1);
    assert_eq!(children[0]["callId"], "delegate-call");
    assert_eq!(children[0]["note"], "正在检查光线");
    assert_eq!(children[0]["phase"]["kind"], "request/start");
    assert_eq!(children[0]["events"].as_array().unwrap().len(), 1);
    assert!(list(&store, "p", "different").unwrap().is_empty());
    assert!(list(&store, "other", "parent").unwrap().is_empty());
    {
        let db = store.db.lock().unwrap();
        update(
            &db,
            "p",
            "child-1",
            "subagent/settled",
            &json!({"stopReason":"completed"}),
        )
        .unwrap();
        start(
            &db,
            "p",
            "child-2",
            "child",
            "followup",
            "message-call",
            "art",
        )
        .unwrap();
        update(
            &db,
            "p",
            "child-2",
            "subagent/settled",
            &json!({"stopReason":"aborted"}),
        )
        .unwrap();
        start(&db, "p", "child-3", "child", "third", "next-call", "art").unwrap();
    }
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    assert_eq!(
        list(&store, "p", "parent").unwrap()[0]["state"],
        "completed"
    );
    assert_eq!(
        list(&store, "p", "followup").unwrap()[0]["state"],
        "cancelled"
    );
    assert_eq!(
        list(&store, "p", "third").unwrap()[0]["state"],
        "interrupted"
    );
    store
        .db
        .lock()
        .unwrap()
        .execute("DELETE FROM projects WHERE id='p'", [])
        .unwrap();
    assert!(list(&store, "p", "parent").unwrap().is_empty());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
