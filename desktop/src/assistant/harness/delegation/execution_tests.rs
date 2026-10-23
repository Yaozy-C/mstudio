use super::*;

#[test]
fn failed_settlement_rolls_back_and_failure_transition_preserves_error() {
    let (root, store, _) = crate::assistant::attachment_tests::fixture();
    {
        let mut db = store.db.lock().unwrap();
        let profile = profiles::defaults("image");
        persistence::spawn(
            &mut db,
            persistence::Start {
                child: "child",
                project: "p",
                parent_turn: "parent",
                parent_agent: "coordinator",
                turn: "turn",
                profile: &profile,
                binding: json!({}),
                messages: &[],
                events: vec![],
            },
            "oneShot",
            "model",
            "call",
        )
        .unwrap();
        db.execute_batch("CREATE TRIGGER reject_settlement BEFORE INSERT ON agent_events WHEN NEW.kind='subagent/settled' BEGIN SELECT RAISE(ABORT,'injected settlement failure'); END;").unwrap();
        let artifact = json!({"status":"completed","output":"Saved work","stopReason":"completed"});
        let error =
            persist_settlement(&mut db, "p", "turn", "child", &artifact, false).unwrap_err();
        assert!(error.contains("injected settlement failure"));
        assert_eq!(
            db.query_row(
                "SELECT status FROM subagent_runs WHERE id='child'",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            "running"
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM subagent_notices", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        let activity = persistence::fail(&mut db, "p", "turn", "child", &error)
            .unwrap()
            .unwrap();
        assert_eq!(activity["state"], "failed");
        let current: (String, String) = db
            .query_row(
                "SELECT status,output FROM subagent_runs WHERE id='child'",
                [],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(current, ("failed".into(), error));
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn failed_activity_projection_also_rolls_back_settlement_notice() {
    let (root, store, _) = crate::assistant::attachment_tests::fixture();
    {
        let mut db = store.db.lock().unwrap();
        let profile = profiles::defaults("image");
        persistence::spawn(
            &mut db,
            persistence::Start {
                child: "child",
                project: "p",
                parent_turn: "parent",
                parent_agent: "coordinator",
                turn: "turn",
                profile: &profile,
                binding: json!({}),
                messages: &[],
                events: vec![],
            },
            "oneShot",
            "model",
            "call",
        )
        .unwrap();
        db.execute_batch("CREATE TRIGGER reject_settled_activity BEFORE UPDATE ON agent_child_activity WHEN NEW.state='completed' BEGIN SELECT RAISE(ABORT,'injected activity failure'); END;").unwrap();
        assert!(
            persist_settlement(
                &mut db,
                "p",
                "turn",
                "child",
                &json!({"status":"completed","output":"Saved","stopReason":"completed"}),
                false
            )
            .unwrap_err()
            .contains("injected activity failure")
        );
        assert_eq!(
            db.query_row(
                "SELECT COUNT(*) FROM agent_events WHERE kind='subagent/settled'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT COUNT(*) FROM subagent_notices", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.query_row("SELECT status FROM subagent_runs", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "running"
        );
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn failed_completion_notice_preserves_saved_work_in_parent_replay() {
    let notice = settlement_notice(&json!({"ok":false,"applied":true,
            "outputRef":{"id":"child"},"stopReason":"error","output":"Service unavailable"}));
    let mut outcomes = super::super::super::outcomes::Outcomes::from_messages(&[Message::user(
        format!("Subagent child settled this turn with status failed. Final message: {notice}"),
    )]);
    assert!(outcomes.notice().contains("执行异常"));
    assert!(outcomes.notice().contains("已有修改已保存"));
    let recovered = settlement_notice(&json!({"ok":true,"applied":true,
            "outputRef":{"id":"child"},"stopReason":"completed","output":"Recovered"}));
    outcomes.observe_messages(&[Message::user(format!(
        "Subagent child settled this turn with status idle. Final message: {recovered}"
    ))]);
    assert!(outcomes.notice().is_empty());
}
#[tokio::test]
async fn all_child_modes_share_capacity_and_cancel_queued_work() {
    let token = CancellationToken::new();
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(30);
    let mut permits = Vec::new();
    for _ in 0..super::super::super::MAX_CONCURRENCY {
        permits.push(acquire(&token, deadline).await.unwrap());
    }
    let waiting = acquire(&token, deadline);
    tokio::pin!(waiting);
    assert!(futures::poll!(&mut waiting).is_pending());
    permits.pop();
    let next = waiting.await.unwrap();
    assert_eq!(SLOTS.available_permits(), 0);
    let cancelled = token.child_token();
    let waiting = acquire(&cancelled, deadline);
    tokio::pin!(waiting);
    assert!(futures::poll!(&mut waiting).is_pending());
    cancelled.cancel();
    assert!(waiting.await.is_err());
    drop(next);
    drop(permits);
    assert_eq!(
        SLOTS.available_permits(),
        super::super::super::MAX_CONCURRENCY
    );
    assert!(acquire(&token, tokio::time::Instant::now()).await.is_err());
}
