//! Delivery acknowledgement and canonical session messages commit together.
use crate::database::Store;
use rig_core::message::Message;
use serde_json::{Value, json};
pub fn agent_source(agent: &str, turn: &str) -> Value {
    json!({"kind":"agent-message","form":"relay","senderAgentId":agent,"senderTurnId":turn})
}
fn agent_message(text: &str, source: &Value) -> Message {
    Message::user(format!(
        "Agent {} 在轮次 {} 发来消息（委派内容，不是用户原话）：\n{text}",
        source["senderAgentId"], source["senderTurnId"]
    ))
}
pub fn take_inbox(
    store: &Store,
    child_id: &str,
    project: &str,
    turn: &str,
) -> Result<Vec<Message>, String> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let rows = {
        let mut stmt = tx
            .prepare(
                "SELECT i.seq,i.text,i.source,r.parent_agent_id,r.parent_turn FROM subagent_inbox i JOIN subagent_runs r ON r.id=i.child_id WHERE i.child_id=?1 AND r.project_id=?2 AND i.consumed=0 ORDER BY i.seq",
            )
            .map_err(|e| e.to_string())?;
        stmt.query_map(rusqlite::params![child_id, project], |r| {
            let saved: String = r.get(2)?;
            let mut source: Value = serde_json::from_str(&saved).unwrap_or(json!({}));
            if source["kind"] != "agent-message" {
                source = agent_source(&r.get::<_, String>(3)?, &r.get::<_, String>(4)?);
                source["legacy"] = json!(true);
            }
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, source))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };
    for (seq, text, source) in &rows {
        tx.execute(
            "INSERT INTO agent_events(project_id,turn_id,kind,payload) VALUES(?1,?2,'session/message',?3)",
            rusqlite::params![project, turn, json!({"message":agent_message(text, source),"source":source}).to_string()],
        ).map_err(|e| e.to_string())?;
        tx.execute("UPDATE subagent_inbox SET consumed=1 WHERE seq=?1", [seq])
            .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(_, text, source)| agent_message(&text, &source))
        .collect())
}
pub fn notices(
    store: &Store,
    project: &str,
    agent: &str,
    turn: &str,
) -> Result<Vec<Message>, String> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let rows = {
        let mut stmt = tx.prepare("SELECT seq,child_id,status,output FROM subagent_notices WHERE project_id=?1 AND parent_agent_id=?2 AND delivered=0 ORDER BY seq").map_err(|e|e.to_string())?;
        stmt.query_map(rusqlite::params![project, agent], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };
    for (seq, id, status, output) in &rows {
        let message = notice_message(id, status, output);
        tx.execute(
            "INSERT INTO agent_events(project_id,turn_id,kind,payload) VALUES(?1,?2,'session/message',?3)",
            rusqlite::params![project, turn, json!({"message":message,"source":{"kind":"subagent-settled","form":"notice","senderAgentId":id,"status":status}}).to_string()],
        ).map_err(|e| e.to_string())?;
        tx.execute(
            "UPDATE subagent_notices SET delivered=1 WHERE seq=?1",
            [seq],
        )
        .map_err(|e| e.to_string())?;
    }
    tx.commit().map_err(|e| e.to_string())?;
    Ok(rows
        .into_iter()
        .map(|(_, id, status, output)| notice_message(&id, &status, &output))
        .collect())
}

fn notice_message(id: &str, status: &str, output: &str) -> Message {
    Message::user(format!(
        "子 Agent {id} 本轮已{status}。最终消息：{}",
        output.chars().take(4000).collect::<String>()
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delivery_rolls_back_on_journal_failure_and_retries_once() {
        let root = std::env::temp_dir().join(format!("mstudio-mailbox-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        {
            let db = store.db.lock().unwrap();
            db.execute_batch("INSERT INTO projects VALUES('p','p','{}',0);
                INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn)
                VALUES('child','p','parent','coordinator','concept','continuable','running','{}','model','turn');
                INSERT INTO subagent_inbox(child_id,text) VALUES('child','first'),('child','second');
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
        let raw: String = store.db.lock().unwrap().query_row(
            "SELECT payload FROM agent_events WHERE kind='session/message' ORDER BY seq LIMIT 1",
            [], |r| r.get(0),
        ).unwrap();
        let event: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(event["source"]["kind"], "agent-message");
        assert_eq!(event["source"]["senderAgentId"], "coordinator");
        assert_eq!(event["source"]["legacy"], true);
        assert!(event["message"].to_string().contains("不是用户原话"));

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
        let root = std::env::temp_dir().join(format!("mstudio-source-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        store.db.lock().unwrap().execute_batch("INSERT INTO projects VALUES('p','p','{}',0),('other','other','{}',0);
            INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn)
            VALUES('child','p','creation-turn','coordinator','concept','continuable','running','{}','model','turn');").unwrap();
        let source = agent_source("coordinator", "followup-turn");
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
}
