use super::*;
use crate::assistant::{harness::session, journal};
use rig_core::message::Message;
#[test]
fn child_continuation_replaces_only_its_own_previous_snapshot() {
    let root =
        std::env::temp_dir().join(format!("mstudio-child-snapshot-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    store.db.lock().unwrap().execute_batch("INSERT INTO projects VALUES('p','p','{}',0);
        INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn)
        VALUES('child','p','parent','coordinator','production','continuable','idle','{}','m','old');").unwrap();
    let binding = json!({"agentId":"production","model":"m"});
    for (turn, child) in [("other", "different-child"), ("old", "child")] {
        session::start(&store, "p", turn, binding.clone(), &[Message::user(turn)]).unwrap();
        journal::append(
            &store,
            "p",
            turn,
            "subagent/settled",
            json!({"childId":child,"status":"idle"}),
        )
        .unwrap();
    }
    let old = session::restore(&store, "p", "old", &binding)
        .unwrap()
        .unwrap();
    session::start(&store, "p", "new", binding.clone(), &old).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("UPDATE subagent_runs SET last_turn='new'", [])
        .unwrap();
    journal::append(
        &store,
        "p",
        "new",
        "subagent/settled",
        json!({"childId":"child","status":"idle"}),
    )
    .unwrap();
    assert!(
        session::restore(&store, "p", "old", &binding)
            .unwrap()
            .is_none()
    );
    assert!(
        session::restore(&store, "p", "other", &binding)
            .unwrap()
            .is_some()
    );
    assert_eq!(
        serde_json::to_value(
            session::restore(&store, "p", "new", &binding)
                .unwrap()
                .unwrap()
        )
        .unwrap(),
        serde_json::to_value(old).unwrap()
    );
    let db = store.db.lock().unwrap();
    assert_eq!(
        session_checkpoint::owner(&db, "p", "old")
            .unwrap()
            .as_deref(),
        Some("production")
    );
    assert!(
        session_checkpoint::owner(&db, "other-project", "old")
            .unwrap()
            .is_none()
    );
    drop(db);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
