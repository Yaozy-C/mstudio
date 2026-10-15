use super::*;
fn take_inbox(
    store: &Store,
    child: &str,
    project: &str,
    turn: &str,
) -> Result<Vec<Message>, String> {
    super::take_inbox(
        store,
        child,
        project,
        turn,
        &crate::assistant::attachment_tests::multimodal(),
        &crate::assistant::profiles::defaults(""),
    )
}
#[test]
fn delivery_rolls_back_on_journal_failure_and_retries_once() {
    let root = std::env::temp_dir().join(format!("mstudio-mailbox-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    {
        let db = store.db.lock().unwrap();
        db.execute_batch("INSERT INTO projects VALUES('p','p','{}',0);
                INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn)
                VALUES('child','p','parent','coordinator','concept','continuable','running','{}','model','turn');
                INSERT INTO subagent_inbox(child_id,text,source) VALUES('child','first','{\"kind\":\"agent-message\",\"senderAgentId\":\"coordinator\"}'),('child','second','{\"kind\":\"agent-message\",\"senderAgentId\":\"coordinator\"}');
                INSERT INTO subagent_notices(child_id,project_id,parent_agent_id,status,output)
                VALUES('child','p','coordinator','idle','done');
                CREATE TRIGGER fail_journal BEFORE INSERT ON agent_events
                WHEN (SELECT count(*) FROM agent_events) >= 1
                BEGIN SELECT RAISE(ABORT,'disk failure'); END;").unwrap();
    }
    // The second write fails: neither the first message nor its acknowledgement may survive.
    assert!(take_inbox(&store, "child", "p", "turn").is_err());
    {
        let db = store.db.lock().unwrap();
        assert_eq!(
            db.query_row("SELECT sum(consumed) FROM subagent_inbox", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM agent_events", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        db.execute_batch("DROP TRIGGER fail_journal;").unwrap();
    }
    assert_eq!(take_inbox(&store, "child", "p", "turn").unwrap().len(), 2);
    assert!(take_inbox(&store, "child", "p", "turn").unwrap().is_empty());
    let raw: String = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT payload FROM agent_events WHERE kind='session/message' ORDER BY seq LIMIT 1",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let event: Value = serde_json::from_str(&raw).unwrap();
    assert_eq!(event["source"]["kind"], "agent-message");
    assert_eq!(event["source"]["senderAgentId"], "coordinator");
    assert!(
        event["message"]
            .to_string()
            .contains("not an original user statement")
    );

    store.db.lock().unwrap().execute_batch("CREATE TRIGGER fail_journal BEFORE INSERT ON agent_events BEGIN SELECT RAISE(ABORT,'disk failure'); END;").unwrap();
    assert!(notices(&store, "p", "coordinator", "parent").is_err());
    {
        let db = store.db.lock().unwrap();
        assert_eq!(
            db.query_row("SELECT delivered FROM subagent_notices", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        db.execute_batch("DROP TRIGGER fail_journal;").unwrap();
    }
    assert_eq!(
        notices(&store, "p", "coordinator", "parent").unwrap().len(),
        1
    );
    drop(store);
    // Reopening uses committed messages; it does not deliver them a second time.
    let store = Store::open(root.clone()).unwrap();
    assert!(take_inbox(&store, "child", "p", "turn").unwrap().is_empty());
    assert!(
        notices(&store, "p", "coordinator", "parent")
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        store
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM agent_events WHERE kind='session/message'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        3
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn sender_provenance_survives_delivery_and_restore_without_becoming_user_evidence() {
    let (root, store, _) = crate::assistant::attachment_tests::fixture();
    store.db.lock().unwrap().execute_batch("INSERT INTO projects VALUES('other','other','{}',0);
            INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn)
            VALUES('child','p','creation-turn','coordinator','concept','continuable','running','{}','model','turn');").unwrap();
    let mut source = agent_source("coordinator", "followup-turn");
    source["originalRequest"] = json!({"refs":[{"kind":"asset","id":"image"}],
            "production":{"instruction":"只修图片","task":{"prompt":"OBSOLETE_PROMPT","parameters":{"duration":6}},
            "referencedTask":{"prompt":"OBSOLETE_REFERENCED_PROMPT"}}});
    store
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO subagent_inbox(child_id,text,source) VALUES('child','只写脚本',?1)",
            [source.to_string()],
        )
        .unwrap();
    let binding = json!({"agentId":"concept"});
    super::super::session::start(&store, "p", "turn", binding.clone(), &[]).unwrap();
    assert!(
        take_inbox(&store, "child", "other", "turn")
            .unwrap()
            .is_empty()
    );
    let delivered = take_inbox(&store, "child", "p", "turn").unwrap();
    let visible = serde_json::to_string(&delivered).unwrap();
    assert!(visible.contains("只修图片") && visible.contains("image"));
    assert!(!visible.contains("OBSOLETE_"));
    let restored = super::super::session::restore(&store, "p", "turn", &binding)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(delivered).unwrap(),
        serde_json::to_value(restored).unwrap()
    );
    let raw: String = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT payload FROM agent_events WHERE kind='session/message'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(
        serde_json::from_str::<Value>(&raw).unwrap()["source"],
        source
    );
    assert!(take_inbox(&store, "child", "p", "turn").unwrap().is_empty());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn notices_follow_task_identity_across_turns_and_leave_other_tasks_unread() {
    let (root, store, _) = crate::assistant::attachment_tests::fixture();
    {
        let db = store.db.lock().unwrap();
        for (turn, task) in [("old", "task-a"), ("same", "task-a"), ("new", "task-b")] {
            let meta = json!({"turnId":turn,"agentId":"coordinator","taskScope":{"taskId":task}});
            db.execute("INSERT INTO agent_messages(project_id,role,content,model,payload,attribution) VALUES('p','user','request','model','{}',?1)", [meta.to_string()]).unwrap();
        }
        db.execute_batch("INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn)
            VALUES('child','p','old','coordinator','concept','oneShot','completed','{}','model','child-turn');
            INSERT INTO subagent_notices(child_id,project_id,parent_agent_id,status,output)
            VALUES('child','p','coordinator','completed','result');").unwrap();
        assert!(!pending(&db, "p", "coordinator", "new").unwrap());
        assert!(pending(&db, "p", "coordinator", "same").unwrap());
    }
    assert!(
        notices(&store, "p", "coordinator", "new")
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        notices(&store, "p", "coordinator", "same").unwrap().len(),
        1
    );
    assert!(
        notices(&store, "p", "coordinator", "same")
            .unwrap()
            .is_empty()
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn initial_and_followup_handoffs_supply_the_same_deduplicated_original_video() {
    use rig_core::message::UserContent;
    let (root, store, doc) = crate::assistant::attachment_tests::fixture();
    let model = crate::assistant::attachment_tests::multimodal();
    let role = crate::assistant::profiles::builtins()
        .into_iter()
        .find(|p| p.id == "video-analyst")
        .unwrap();
    let source = json!({"kind":"agent-message","senderAgentId":"coordinator","senderTurnId":"parent",
        "originalRequest":{"production":{"instruction":"字".repeat(8000)}},"artifacts":[],"expectedGenerations":[],
        "refs":[{"kind":"asset","id":"video"},{"kind":"asset","id":"video"}]});
    let initial =
        super::super::handoff_artifacts::message(&store, &doc, &model, &role, "拆解声画", &source)
            .unwrap();
    {
        let db = store.db.lock().unwrap();
        db.execute_batch("INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn)
            VALUES('child','p','parent','coordinator','video-analyst','continuable','running','{}','model','child-turn');").unwrap();
        db.execute(
            "INSERT INTO subagent_inbox(child_id,text,source) VALUES('child','拆解声画',?1)",
            [source.to_string()],
        )
        .unwrap();
    }
    let followup = super::take_inbox(&store, "child", "p", "child-turn", &model, &role).unwrap();
    assert_eq!(followup, vec![initial]);
    let Message::User { content } = &followup[0] else {
        panic!("Expected user message")
    };
    assert_eq!(
        content
            .iter()
            .filter(|p| matches!(p, UserContent::Video(_)))
            .count(),
        1
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
