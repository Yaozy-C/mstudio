use super::*;
use crate::assistant::{attachment_tests::fixture, history, parent_activation};
use tokio_util::sync::CancellationToken;

fn begin(store: &Store) {
    let request = json!({"refs":[],"production":{"projectId":"p","instruction":"Write a script"}});
    history::begin(
        store,
        "p",
        "parent",
        &request,
        "model",
        &json!({"turnId":"parent"}),
    )
    .unwrap();
    store.db.lock().unwrap().execute_batch("INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn)
        VALUES('child','p','parent','coordinator','writer','oneShot','running','{}','model','child-turn');").unwrap();
}

#[test]
fn wait_expiry_keeps_parent_active_until_child_and_result_delivery_finish() {
    let (root, store, _) = fixture();
    begin(&store);
    let state = finish_saved(
        &store,
        "p",
        "parent",
        &Ok("Waiting for the writer".into()),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(state.status, "waiting");
    assert_eq!(read(&store, "p").unwrap().unwrap().status, "waiting");
    assert_eq!(
        store
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM agent_events WHERE turn_id='parent' AND kind='turn/end'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        0
    );
    {
        let db = store.db.lock().unwrap();
        db.execute_batch("UPDATE subagent_runs SET status='completed' WHERE id='child';
            INSERT INTO subagent_notices(child_id,project_id,parent_agent_id,status,output) VALUES('child','p','coordinator','completed','saved script');").unwrap();
        assert!(
            pending(&db, "p", "parent").unwrap(),
            "child completion alone is not parent delivery"
        );
    }
    parent_activation::reopen_response(&store, "p", "parent").unwrap();
    assert_eq!(read(&store, "p").unwrap().unwrap().status, "running");
    store
        .db
        .lock()
        .unwrap()
        .execute("UPDATE subagent_notices SET delivered=1", [])
        .unwrap();
    let state = finish_saved(
        &store,
        "p",
        "parent",
        &Ok("Here is the saved script".into()),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(state.status, "completed");
    assert_eq!(state.text, "Here is the saved script");
    assert_eq!(
        history::read(&store, "p").unwrap().len(),
        2,
        "background delivery updates the same response"
    );
    assert_eq!(
        store
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT count(*) FROM agent_events WHERE turn_id='parent' AND kind='turn/end'",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
        1
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn errors_and_cancellation_do_not_become_waiting_even_with_running_children() {
    for (cancelled, status) in [(false, "failed"), (true, "cancelled")] {
        let (root, store, _) = fixture();
        begin(&store);
        let token = CancellationToken::new();
        if cancelled {
            token.cancel();
        }
        let state = finish_saved(&store, "p", "parent", &Err("Stopped".into()), &token).unwrap();
        assert_eq!(state.status, status);
        assert!(parent_activation::reopen_response(&store, "p", "parent").is_err());
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}

#[test]
fn waiting_parent_survives_reload_and_becomes_interrupted_after_process_restart() {
    let (root, store, _) = fixture();
    begin(&store);
    finish_saved(
        &store,
        "p",
        "parent",
        &Ok("Still waiting".into()),
        &CancellationToken::new(),
    )
    .unwrap();
    assert_eq!(read(&store, "p").unwrap().unwrap().text, "Still waiting");
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    let state = read(&store, "p").unwrap().unwrap();
    assert_eq!(state.status, "interrupted");
    assert!(state.error.is_some());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn stopping_parked_parent_is_durable_and_late_delivery_cannot_reopen_it() {
    let (root, store, _) = fixture();
    begin(&store);
    finish_saved(
        &store,
        "p",
        "parent",
        &Ok("Waiting".into()),
        &CancellationToken::new(),
    )
    .unwrap();
    let states = end_saved(&store, "p", None, "cancelled", "Stopped").unwrap();
    assert_eq!(states.len(), 1);
    assert_eq!(read(&store, "p").unwrap().unwrap().status, "cancelled");
    assert!(parent_activation::reopen_response(&store, "p", "parent").is_err());
    assert!(
        end_saved(&store, "p", Some("parent"), "failed", "Late failure")
            .unwrap()
            .is_empty()
    );
    assert!(states[0].revision > 0);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn background_recovery_failure_ends_waiting_without_erasing_saved_output() {
    let (root, store, _) = fixture();
    begin(&store);
    finish_saved(
        &store,
        "p",
        "parent",
        &Ok("Saved progress".into()),
        &CancellationToken::new(),
    )
    .unwrap();
    let states = end_saved(&store, "p", Some("parent"), "failed", "Session unavailable").unwrap();
    assert_eq!(states[0].status, "failed");
    assert_eq!(states[0].text, "Saved progress");
    assert_eq!(states[0].error.as_deref(), Some("Session unavailable"));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
