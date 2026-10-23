use super::*;

fn start<'a>(profile: &'a profiles::AgentProfile, turn: &'a str) -> Start<'a> {
    Start {
        child: "child",
        project: "p",
        parent_turn: "parent",
        parent_agent: "coordinator",
        turn,
        profile,
        binding: json!({}),
        messages: &[],
        events: vec![("subagent/context", json!({"source":"current"}))],
    }
}

#[test]
fn spawn_failure_rolls_back_run_session_and_observations() {
    let (root, store, _) = crate::assistant::attachment_tests::fixture();
    {
        let mut db = store.db.lock().unwrap();
        db.execute_batch("CREATE TRIGGER reject_child_activity BEFORE INSERT ON agent_child_activity BEGIN SELECT RAISE(ABORT,'injected child activity failure'); END;").unwrap();
        let profile = profiles::defaults("image");
        let error = spawn(
            &mut db,
            start(&profile, "first"),
            "oneShot",
            "model",
            "call",
        )
        .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("injected child activity failure")
        );
        for table in ["subagent_runs", "agent_events", "project_observations"] {
            let count: i64 = db
                .query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))
                .unwrap();
            assert_eq!(count, 0, "{table} retained a partial start");
        }
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn continuation_failure_preserves_idle_run_and_accepted_message() {
    let (root, store, _) = crate::assistant::attachment_tests::fixture();
    {
        let mut db = store.db.lock().unwrap();
        let profile = profiles::defaults("image");
        spawn(
            &mut db,
            start(&profile, "first"),
            "continuable",
            "model",
            "call",
        )
        .unwrap();
        db.execute(
            "UPDATE subagent_runs SET status='idle' WHERE id='child'",
            [],
        )
        .unwrap();
        db.execute(
            "INSERT INTO subagent_inbox(child_id,text,source) VALUES('child','next',?1)",
            [json!({"senderTurnId":"parent","callId":"next-call"}).to_string()],
        )
        .unwrap();
        let before: i64 = db
            .query_row("SELECT COUNT(*) FROM agent_events", [], |r| r.get(0))
            .unwrap();
        db.execute_batch("CREATE TRIGGER reject_next_activity BEFORE INSERT ON agent_child_activity WHEN NEW.turn_id='next' BEGIN SELECT RAISE(ABORT,'injected resume failure'); END;").unwrap();
        assert!(resume(&mut db, start(&profile, "next"), "first").is_err());
        let current: (String, String) = db
            .query_row(
                "SELECT status,last_turn FROM subagent_runs WHERE id='child'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(current, ("idle".into(), "first".into()));
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM agent_events", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            before
        );
        assert_eq!(
            db.query_row("SELECT consumed FROM subagent_inbox", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        db.execute_batch("DROP TRIGGER reject_next_activity")
            .unwrap();
        assert!(resume(&mut db, start(&profile, "next"), "first").unwrap());
        assert!(!resume(&mut db, start(&profile, "duplicate"), "first").unwrap());
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
