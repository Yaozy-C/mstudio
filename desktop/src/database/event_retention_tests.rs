use super::*;
use crate::assistant::{history, journal};

fn store() -> Store {
    let root = std::env::temp_dir().join(format!("mstudio-retention-{}", mstudio::media::id()));
    let store = Store::open(root).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','p','{}',0)", [])
        .unwrap();
    store
}
fn begin(store: &Store, turn: &str) {
    let request = json!({"production":{"projectId":"p","instruction":"request"},"refs":[]});
    history::begin(store, "p", turn, &request, "model", &json!({"turnId":turn})).unwrap();
}
fn legacy(db: &Connection, turn: &str, kind: &str) {
    db.execute(
        "INSERT INTO agent_events(project_id,turn_id,kind,payload) VALUES('p',?1,?2,'{}')",
        params![turn, kind],
    )
    .unwrap();
}
fn count(db: &Connection, turn: &str) -> i64 {
    db.query_row(
        "SELECT count(*) FROM agent_events WHERE turn_id=?1",
        [turn],
        |r| r.get(0),
    )
    .unwrap()
}
#[test]
fn live_progress_keeps_crash_checkpoint_without_appending_fragments() {
    let store = store();
    begin(&store, "t");
    for n in 1..=50 {
        journal::append(
            &store,
            "p",
            "t",
            "assistant/partial",
            json!({"text":"x".repeat(n),"delta":"x"}),
        )
        .unwrap();
        journal::append(&store, "p", "t", "request/start", json!({})).unwrap();
    }
    assert_eq!(count(&store.db.lock().unwrap(), "t"), 0);
    assert_eq!(
        history::read(&store, "p").unwrap()[1].content,
        "x".repeat(50)
    );
    let root = store.root.clone();
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    let messages = history::read(&store, "p").unwrap();
    assert_eq!(messages[1].content, "x".repeat(50));
    assert_eq!(
        messages[1].attribution.as_ref().unwrap()["status"],
        "interrupted"
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn migration_keeps_recovery_usage_errors_unknown_events_and_active_turns() {
    let store = store();
    begin(&store, "active");
    {
        let db = store.db.lock().unwrap();
        db.execute_batch("DELETE FROM database_migrations WHERE version=2;
            INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn)
            VALUES('child','p','parent','coordinator','production','continuable','running','{}','m','child-turn');").unwrap();
        for kind in event_retention::TRANSIENT {
            for turn in ["done", "active", "child-turn"] {
                legacy(&db, turn, kind);
            }
        }
        for kind in [
            "session/start",
            "session/message",
            "session/compaction",
            "session/reset",
            "image/offload",
            "tool/call",
            "tool/result",
            "turn/end",
            "subagent/settled",
            "request/usage",
            "compaction/end",
            "request/retry",
            "model/stop",
            "future/event",
        ] {
            legacy(&db, "keep", kind);
        }
        let mut payload = json!({"diagnostic":"x".repeat(10000)});
        legacy(&db, "done", "request/context");
        let seq = db.last_insert_rowid();
        blobs::pack(&db, blobs::Owner::Event(seq), &mut payload).unwrap();
        db.execute(
            "UPDATE agent_events SET payload=?1 WHERE seq=?2",
            params![payload.to_string(), seq],
        )
        .unwrap();
        event_retention::migrate(&db).unwrap();
        event_retention::migrate(&db).unwrap();
        assert_eq!(count(&db, "done"), 0);
        assert_eq!(count(&db, "keep"), 14);
        assert_eq!(
            count(&db, "active"),
            event_retention::TRANSIENT.len() as i64
        );
        assert_eq!(
            count(&db, "child-turn"),
            event_retention::TRANSIENT.len() as i64
        );
        assert_eq!(
            db.query_row("SELECT count(*) FROM content_blobs", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            0
        );
    }
    history::finish(&store, "p", "active", "answer", "completed", None).unwrap();
    journal::append(
        &store,
        "p",
        "active",
        "turn/end",
        json!({"status":"completed"}),
    )
    .unwrap();
    assert_eq!(count(&store.db.lock().unwrap(), "active"), 1);
    store
        .db
        .lock()
        .unwrap()
        .execute("UPDATE subagent_runs SET status='idle'", [])
        .unwrap();
    journal::append(
        &store,
        "p",
        "child-turn",
        "subagent/settled",
        json!({"status":"idle"}),
    )
    .unwrap();
    assert_eq!(count(&store.db.lock().unwrap(), "child-turn"), 1);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn failed_cleanup_rolls_back_and_can_retry() {
    let store = store();
    {
        let db = store.db.lock().unwrap();
        db.execute("DELETE FROM database_migrations WHERE version=2", [])
            .unwrap();
        legacy(&db, "done", "step/start");
        legacy(&db, "done", "step/end");
        db.execute_batch("CREATE TRIGGER reject_cleanup BEFORE DELETE ON agent_events WHEN OLD.kind='step/end' BEGIN SELECT RAISE(ABORT,'test failure'); END;").unwrap();
        assert!(event_retention::migrate(&db).is_err());
        assert_eq!(count(&db, "done"), 2);
        assert_eq!(
            db.query_row(
                "SELECT count(*) FROM database_migrations WHERE version=2",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        db.execute_batch("DROP TRIGGER reject_cleanup;").unwrap();
        event_retention::migrate(&db).unwrap();
        assert_eq!(count(&db, "done"), 0);
    }
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
#[ignore = "requires MSTUDIO_RETENTION_AUDIT pointing to a disposable SQLite copy"]
fn existing_database_retains_every_durable_event() {
    let path = std::env::var("MSTUDIO_RETENTION_AUDIT").unwrap();
    let db = Connection::open(&path).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let rows = {
        let mut stmt = db
            .prepare(
                "SELECT seq,kind,payload,project_id,turn_id,created FROM agent_events ORDER BY seq",
            )
            .unwrap();
        stmt.query_map([], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)?,
            ))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
    };
    event_retention::migrate(&db).unwrap();
    let mut retained = 0;
    for (seq, kind, payload, project, turn, created) in &rows {
        if event_retention::transient(kind) {
            continue;
        }
        let row = db
            .query_row(
                "SELECT kind,payload,project_id,turn_id,created FROM agent_events WHERE seq=?1",
                [seq],
                |r| {
                    Ok((
                        r.get::<_, String>(0)?,
                        r.get::<_, String>(1)?,
                        r.get::<_, String>(2)?,
                        r.get::<_, String>(3)?,
                        r.get::<_, i64>(4)?,
                    ))
                },
            )
            .unwrap();
        assert_eq!(
            row,
            (
                kind.clone(),
                payload.clone(),
                project.clone(),
                turn.clone(),
                *created
            )
        );
        retained += 1;
    }
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    println!(
        "{}",
        json!({"before":rows.len(),"retainedVerified":retained,"after":db.query_row("SELECT count(*) FROM agent_events",[],|r|r.get::<_,i64>(0)).unwrap(),"databaseBytes":std::fs::metadata(path).unwrap().len()})
    );
}
