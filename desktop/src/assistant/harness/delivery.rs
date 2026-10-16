//! Generation deliverables are verified from receipts, separately from run termination.
use serde_json::{Value, json};

pub fn schema() -> Value {
    json!({"type":"array","maxItems":24,"description":"Declare the generation task deliverables; this is a delivery checklist, not a reason to bundle independent work. One entry per requested output task, not per shot. For a merged video use its anchor shot targetId once. Pass [] for research or prompt-only work. The host checks actual task receipts, not the specialist's prose.","items":{"type":"object","properties":{"targetId":{"type":"string","minLength":1,"maxLength":200},"kind":{"type":"string","enum":["image","video"]}},"required":["targetId","kind"],"additionalProperties":false}})
}

pub fn assess(expected: &Value, tasks: &Value) -> Value {
    let Some(expected) = expected.as_array().filter(|a| !a.is_empty()) else {
        return json!({"state":"not_specified","complete":null});
    };
    let items: Vec<_> = expected.iter().map(|item| {
        let found: Vec<_> = tasks.as_array().into_iter().flatten().filter(|task|
            task["targetId"] == item["targetId"] && task["kind"] == item["kind"]
            && task["id"].as_str().is_some_and(|id| !id.is_empty())
        ).collect();
        json!({"targetId":item["targetId"],"kind":item["kind"],
            "created":!found.is_empty(),"tasks":found,
            "next":if found.is_empty(){"Create the missing authorized task, or report a specific hard blocker. A saved prompt or quality concern is not a creation receipt."}else{"Use the returned task status; do not duplicate this task or bypass confirmation."}})
    }).collect();
    let complete = items.iter().all(|item| item["created"] == true);
    json!({"state":if complete{"created"}else{"incomplete"},"complete":complete,"items":items,
        "meaning":"Checks task creation only. completed/ok describe the agent run, not media completion. AWAITING_CONFIRMATION means created but not submitted to the service."})
}

pub fn for_turn(store: &crate::database::Store, project: &str, turn: &str, tasks: &Value) -> Value {
    use rusqlite::OptionalExtension;
    let expected = store.db.lock().unwrap().query_row(
        "SELECT json_extract(payload,'$.expectedGenerations') FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind='subagent/requirements' ORDER BY seq DESC LIMIT 1",
        rusqlite::params![project, turn], |r| r.get::<_, Option<String>>(0),
    ).optional();
    match expected {
        Ok(value) => assess(
            &value
                .flatten()
                .and_then(|s| serde_json::from_str::<Value>(&s).ok())
                .unwrap_or(Value::Null),
            tasks,
        ),
        Err(error) => json!({"state":"unverified","complete":false,"error":error.to_string()}),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn partial_generation_is_not_a_complete_delivery() {
        let expected = json!([{"targetId":"s3","kind":"video"},{"targetId":"s5","kind":"video"}]);
        let tasks =
            json!([{"id":"five","targetId":"s5","kind":"video","status":"AWAITING_CONFIRMATION"}]);
        let result = assess(&expected, &tasks);
        assert_eq!(result["complete"], false);
        assert_eq!(result["items"][0]["created"], false);
        assert_eq!(
            result["items"][1]["tasks"][0]["status"],
            "AWAITING_CONFIRMATION"
        );
        assert_eq!(
            assess(&json!([{"targetId":"s5","kind":"video"}]), &tasks)["complete"],
            true
        );
        assert_eq!(assess(&expected, &json!([]))["complete"], false);
        assert_eq!(
            assess(&json!([{"targetId":"s5","kind":"image"}]), &tasks)["complete"],
            false
        );
        assert!(assess(&Value::Null, &tasks)["complete"].is_null());
    }
    #[test]
    fn large_receipts_keep_complete_delivery_control_fields() {
        use rig_core::message::{ToolCall, ToolFunction};
        let expected = json!(
            (0..24)
                .map(|n| json!({"targetId":format!("s{n}"),"kind":"video"}))
                .collect::<Vec<_>>()
        );
        let delivery = assess(&expected, &json!([]));
        let value =
            json!({"childId":"c","ok":true,"delivery":delivery,"changes":"x".repeat(20000)});
        let call = ToolCall::from_wire(
            "d",
            ToolFunction::new("mstudio_delegate".try_into().unwrap(), json!({})),
        );
        let projected = super::super::tool_output::project(&call, &value, Some("turn"));
        assert_eq!(projected["delivery"], delivery);
    }
    #[test]
    fn requirements_survive_settlement_retention_and_drive_verification() {
        use crate::{assistant::journal, database::Store};
        let root = std::env::temp_dir().join(format!("delivery-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        crate::projects::write_document(
            &store,
            json!({"id":"p","name":"p","assets":[],"nodes":[]}),
            true,
        )
        .unwrap();
        journal::append(
            &store,
            "p",
            "turn",
            "subagent/requirements",
            json!({"expectedGenerations":[{"targetId":"s3","kind":"video"}]}),
        )
        .unwrap();
        crate::database::event_retention::finish(&store.db.lock().unwrap(), "p", "turn").unwrap();
        assert_eq!(for_turn(&store, "p", "turn", &json!([]))["complete"], false);
        assert_eq!(
            for_turn(
                &store,
                "p",
                "turn",
                &json!([{"id":"run","targetId":"s3","kind":"video"}])
            )["complete"],
            true
        );
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn followup_inbox_persists_its_remaining_deliverables() {
        use crate::database::Store;
        let root = std::env::temp_dir().join(format!("delivery-followup-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        {
            let db = store.db.lock().unwrap();
            db.execute_batch("INSERT INTO projects VALUES('p','p','{}',0); INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn) VALUES('child','p','parent','coordinator','production','continuable','running','{}','model','next');").unwrap();
            let source = json!({"kind":"agent-message","expectedGenerations":[{"targetId":"s4","kind":"video"}]});
            db.execute("INSERT INTO subagent_inbox(child_id,text,source) VALUES('child','finish remaining video',?1)", [source.to_string()]).unwrap();
        }
        let messages = super::super::mailbox::take_inbox(
            &store,
            "child",
            "p",
            "next",
            &Default::default(),
            &crate::assistant::profiles::defaults(""),
        )
        .unwrap();
        assert!(
            serde_json::to_string(&messages)
                .unwrap()
                .contains("Required generation task receipts")
        );
        assert_eq!(
            for_turn(&store, "p", "next", &json!([]))["items"][0]["targetId"],
            "s4"
        );
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
