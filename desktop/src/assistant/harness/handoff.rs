//! DSH-style one-time completed-turn seed; role composition remains child-owned.
use crate::database::Store;
use rig_core::message::Message;
use serde_json::Value;

/// Preserve provider messages (including tool pairs/signatures) while installing this role.
pub fn messages(system: String, history: &[Message]) -> Vec<Message> {
    let mut messages = vec![Message::System { content: system }];
    messages.extend(
        history
            .iter()
            .filter(|m| !matches!(m, Message::System { .. }))
            .cloned(),
    );
    messages
}

/// Capture only a closed parent turn, never the current/retried turn's partial transcript.
/// Each session/start already contains the prior series, so restoring the last closed turn
/// gives the complete retained prefix, including canonical compaction and image offloads.
pub fn completed(
    store: &Store,
    project: &str,
    current_turn: &str,
    binding: &Value,
) -> Result<Vec<Message>, String> {
    let candidates = {
        let db = store.db.lock().unwrap();
        let mut stmt = db.prepare(
            "SELECT e.turn_id,e.seq,json_extract(s.payload,'$.binding') FROM agent_events e JOIN agent_events s
             ON s.project_id=e.project_id AND s.turn_id=e.turn_id AND s.kind='session/start'
             WHERE e.project_id=?1 AND e.turn_id!=?2 AND e.kind='turn/end' AND s.seq<e.seq AND COALESCE(json_extract(s.payload,'$.checkpointSeq'),s.seq)<=e.seq
             AND s.seq>COALESCE((SELECT MAX(seq) FROM agent_events WHERE project_id=?1 AND kind='session/reset'),0)
             ORDER BY e.seq DESC,s.seq DESC"
        ).map_err(|e| e.to_string())?;
        stmt.query_map(rusqlite::params![project, current_turn], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| e.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())?
    };
    for (turn, end, payload) in candidates {
        let start: Value = serde_json::from_str(&payload).map_err(|e| e.to_string())?;
        if super::binding::compatible(&start, binding) {
            return Ok(
                super::session::restore_through(store, project, &turn, binding, end)?
                    .unwrap_or_default(),
            );
        }
    }
    Ok(vec![])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::{harness::session, journal};
    use rig_core::message::{AssistantContent, ToolCall, ToolFunction};
    use serde_json::json;

    fn transcript() -> Vec<Message> {
        let call = ToolCall::from_wire(
            "saved-call",
            ToolFunction {
                name: "mstudio_edit".into(),
                arguments: json!({"revision":1}),
            },
        );
        vec![
            Message::System {
                content: "parent-role".into(),
            },
            Message::user("选第二个"),
            Message::Assistant {
                id: Some("provider-id".into()),
                content: vec![AssistantContent::ToolCall(call.clone())],
            },
            session::result_message(&call, &json!({"applied":true})),
            Message::assistant("saved answer"),
        ]
    }
    #[test]
    fn independent_role_keeps_exact_tool_pairs_and_does_not_mutate_parent() {
        let history = transcript();
        let old = serde_json::to_value(&history).unwrap();
        let child = messages("child-role".into(), &history);
        assert_eq!(
            serde_json::to_value(&child[1..]).unwrap(),
            serde_json::to_value(&history[1..]).unwrap()
        );
        assert!(matches!(&child[0], Message::System {content} if content=="child-role"));
        assert_eq!(serde_json::to_value(&history).unwrap(), old);
        assert_eq!(messages("spawn-role".into(), &[]).len(), 1);
        // A continuation uses the child's own transcript, including its new tool results.
        let mut own = child.clone();
        own.push(Message::assistant("child-only-output"));
        let resumed = messages("updated-child-role".into(), &own);
        assert_eq!(
            serde_json::to_value(&resumed[1..]).unwrap(),
            serde_json::to_value(&own[1..]).unwrap()
        );
    }
    #[test]
    fn fork_only_uses_closed_turn_snapshot_and_respects_reset_project_and_route() {
        let root = std::env::temp_dir().join(format!("mstudio-fork-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        store
            .db
            .lock()
            .unwrap()
            .execute_batch("INSERT INTO projects VALUES('p','p','{}',0),('other','other','{}',0);")
            .unwrap();
        let binding = json!({"agentId":"coordinator","model":"m","revision":1});
        session::start(&store, "p", "finished", binding.clone(), &transcript()).unwrap();
        assert!(
            completed(&store, "p", "active", &binding)
                .unwrap()
                .is_empty()
        );
        journal::append(
            &store,
            "p",
            "finished",
            "turn/end",
            json!({"status":"completed"}),
        )
        .unwrap();
        let seed = completed(&store, "p", "active", &binding).unwrap();
        assert_eq!(
            serde_json::to_value(&seed).unwrap(),
            serde_json::to_value(transcript()).unwrap()
        );
        // A late event after turn/end and a current/retry transcript must never enter the seed.
        journal::append(
            &store,
            "p",
            "finished",
            "session/message",
            json!({"message":Message::user("late")}),
        )
        .unwrap();
        session::start(
            &store,
            "p",
            "active",
            binding.clone(),
            &[Message::user("in flight")],
        )
        .unwrap();
        journal::append(
            &store,
            "p",
            "active",
            "turn/end",
            json!({"status":"failed"}),
        )
        .unwrap();
        session::start(
            &store,
            "p",
            "active",
            binding.clone(),
            &[Message::user("retry in flight")],
        )
        .unwrap();
        assert_eq!(
            serde_json::to_value(completed(&store, "p", "active", &binding).unwrap()).unwrap(),
            serde_json::to_value(&seed).unwrap()
        );
        assert!(
            completed(&store, "other", "active", &binding)
                .unwrap()
                .is_empty()
        );
        let other = json!({"agentId":"concept","model":"m","revision":1});
        assert!(completed(&store, "p", "active", &other).unwrap().is_empty());
        let updated = json!({"agentId":"coordinator","model":"m","revision":2});
        assert_eq!(
            completed(&store, "p", "active", &updated).unwrap().len(),
            seed.len()
        );
        journal::append(&store, "p", "reset", "session/reset", json!({})).unwrap();
        assert!(
            completed(&store, "p", "active", &binding)
                .unwrap()
                .is_empty()
        );
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
