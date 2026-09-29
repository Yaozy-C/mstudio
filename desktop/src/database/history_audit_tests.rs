//! Run against a disposable copy before applying the migration to the desktop library.
use super::*;
use crate::assistant::harness::session;
use sha2::{Digest, Sha256};

fn fingerprint(value: Option<Vec<rig_core::message::Message>>) -> Option<String> {
    value.map(|messages| {
        format!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(&messages).unwrap())
        )
    })
}

#[test]
#[ignore = "requires MSTUDIO_HISTORY_AUDIT pointing to a disposable directory"]
fn verify_existing_sessions_and_chat_survive_cleanup() {
    let root = PathBuf::from(std::env::var("MSTUDIO_HISTORY_AUDIT").unwrap());
    let db = Connection::open(root.join("mstudio.sqlite3")).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON").unwrap();
    let location = crate::storage::load(&db, &root).unwrap();
    let store = Store {
        root: root.clone(),
        db: Mutex::new(db),
        files: Default::default(),
        imports: Mutex::new(()),
        location: std::sync::RwLock::new(location),
    };
    let candidates = {
        let db = store.db.lock().unwrap();
        let mut stmt=db.prepare("SELECT project_id,turn_id,json_extract(payload,'$.binding') FROM agent_events WHERE kind='session/start' GROUP BY project_id,turn_id").unwrap();
        stmt.query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .unwrap()
        .collect::<rusqlite::Result<Vec<_>>>()
        .unwrap()
    };
    let mut before = Vec::new();
    for (project, turn, raw) in candidates {
        let binding: Value = serde_json::from_str(&raw).unwrap();
        let messages = session::restore(&store, &project, &turn, &binding);
        before.push((project, turn, binding, messages.map(fingerprint)));
    }
    println!("Captured {} recovery fingerprints", before.len());
    let (chat, jobs, usage, initial) = {
        let db = store.db.lock().unwrap();
        let chat:String=db.query_row("SELECT json_group_array(json_object('id',id,'project',project_id,'role',role,'content',content,'payload',payload,'meta',attribution)) FROM agent_messages",[],|r|r.get(0)).unwrap();
        let jobs: String = db
            .query_row(
                "SELECT json_group_array(json_object('id',id,'data',data)) FROM jobs",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let usage:i64=db.query_row("SELECT COALESCE(sum(json_extract(payload,'$.totalTokens')),0) FROM agent_events WHERE kind IN ('request/usage','compaction/end','turn/usage')",[],|r|r.get(0)).unwrap();
        let initial: i64 = db
            .query_row("SELECT count(*) FROM agent_events", [], |r| r.get(0))
            .unwrap();
        history_cleanup::startup(&db).unwrap();
        history_cleanup::startup(&db).unwrap();
        (chat, jobs, usage, initial)
    };
    let mut verified = 0;
    let mut replaced = 0;
    for (project, turn, binding, old) in before {
        let after = session::restore(&store, &project, &turn, &binding).map(fingerprint);
        if after == Ok(None) && old != Ok(None) {
            let db = store.db.lock().unwrap();
            let completed:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM agent_messages WHERE project_id=?1 AND role='assistant' AND json_extract(attribution,'$.turnId')=?2 AND json_extract(attribution,'$.status')='completed') OR EXISTS(SELECT 1 FROM agent_events e JOIN subagent_runs r ON r.project_id=e.project_id AND r.id=json_extract(e.payload,'$.childId') WHERE e.project_id=?1 AND e.turn_id=?2 AND e.kind='subagent/settled' AND r.last_turn!=?2)",params![project,turn],|r|r.get(0)).unwrap();
            assert!(completed, "Non-completed recovery state was removed");
            replaced += 1;
            continue;
        }
        assert_eq!(after, old, "session {project}/{turn}");
        verified += 1;
    }
    let db = store.db.lock().unwrap();
    assert_eq!(chat,db.query_row("SELECT json_group_array(json_object('id',id,'project',project_id,'role',role,'content',content,'payload',payload,'meta',attribution)) FROM agent_messages",[],|r|r.get::<_,String>(0)).unwrap());
    assert_eq!(
        jobs,
        db.query_row(
            "SELECT json_group_array(json_object('id',id,'data',data)) FROM jobs",
            [],
            |r| r.get::<_, String>(0)
        )
        .unwrap()
    );
    assert_eq!(usage,db.query_row("SELECT COALESCE(sum(json_extract(payload,'$.totalTokens')),0) FROM agent_events WHERE kind='turn/usage'",[],|r|r.get::<_,i64>(0)).unwrap());
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
        json!({"initialEvents":initial,"events":db.query_row("SELECT count(*) FROM agent_events",[],|r|r.get::<_,i64>(0)).unwrap(),"sessionsVerified":verified,"superseded":replaced,"databaseBytes":std::fs::metadata(root.join("mstudio.sqlite3")).unwrap().len()})
    );
}
