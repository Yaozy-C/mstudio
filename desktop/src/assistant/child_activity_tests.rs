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
        let value=update(&db,"p","child-1","tool/call",&json!({"name":"mstudio_inspect","arguments":{"section":"assets","text":"private prompt"}})).unwrap().unwrap();
        assert_eq!(value["phase"]["payload"]["name"], "mstudio_inspect");
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
        json!({"callId":"read","name":"mstudio_inspect"}),
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

#[test]
fn historical_handoffs_link_by_child_identity_without_guessing_by_role() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE projects(id TEXT PRIMARY KEY); CREATE TABLE subagent_runs(id TEXT PRIMARY KEY,project_id TEXT,parent_turn TEXT,agent_id TEXT,mode TEXT,status TEXT,last_turn TEXT,created INTEGER,updated INTEGER); CREATE TABLE agent_events(project_id TEXT,turn_id TEXT,kind TEXT,payload TEXT); INSERT INTO projects VALUES('p'); INSERT INTO subagent_runs VALUES('child','p','parent','art','oneShot','completed','child-turn',10,280); INSERT INTO subagent_runs VALUES('other','p','parent','art','oneShot','completed','other-turn',11,290);").unwrap();
    db.execute(
        "INSERT INTO agent_events VALUES('p','parent','tool/result',?1)",
        [
            json!({"callId":"delegate","result":{"childId":"child","stopReason":"completed"}})
                .to_string(),
        ],
    )
    .unwrap();
    init(&db).unwrap();
    assert_eq!(
        snapshot(&db, "p", "child-turn").unwrap().unwrap()["callId"],
        "delegate"
    );
    assert_eq!(
        snapshot(&db, "p", "child-turn").unwrap().unwrap()["state"],
        "completed"
    );
    assert!(snapshot(&db, "p", "other-turn").unwrap().is_none());
    init(&db).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM agent_child_activity", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
