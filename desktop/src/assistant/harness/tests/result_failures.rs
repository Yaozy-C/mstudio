use super::{super::*, support::*};
use rig_core::{
    completion::ToolDefinition,
    message::{Message, ToolCall},
};
use serde_json::{Value, json};
use tokio_util::sync::CancellationToken;

struct SavedHost<'a> {
    store: &'a crate::database::Store,
    token: CancellationToken,
}
impl Host for SavedHost<'_> {
    fn token(&self) -> &CancellationToken {
        &self.token
    }
    fn definitions(&self) -> Result<Vec<ToolDefinition>, String> {
        Ok(vec![])
    }
    fn parallel_safe(&self, _: &ToolCall) -> bool {
        false
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        crate::assistant::journal::append(self.store, "p", "turn", kind, value)
            .map_err(|error| error.to_string())
    }
    async fn execute(&self, _: &ToolCall) -> Value {
        panic!("skipped call executed")
    }
}

#[test]
fn skipped_call_can_be_read_with_its_canonical_error() {
    let (root, store, _) = crate::assistant::attachment_tests::fixture();
    let host = SavedHost {
        store: &store,
        token: CancellationToken::new(),
    };
    scheduler::skip(
        &host,
        &mut session::Session::new(vec![]),
        &[call("skip", "write", json!({}))],
        "MODEL_OUTPUT_INCOMPLETE",
        "Not executed",
    )
    .unwrap();
    let value = tool_output::read_page(&store, "p", "turn", "skip", 0);
    let saved: Value = serde_json::from_str(value["text"].as_str().unwrap()).unwrap();
    assert_eq!(saved["code"], "MODEL_OUTPUT_INCOMPLETE");
    assert_eq!(saved["error"], "Not executed");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn missing_canonical_value_and_storage_failures_are_not_not_found() {
    let (root, store, _) = crate::assistant::attachment_tests::fixture();
    assert_eq!(
        tool_output::read_page(&store, "p", "t", "absent", 0)["code"],
        "RESULT_NOT_FOUND"
    );
    crate::assistant::journal::append(
        &store,
        "p",
        "t",
        "tool/result",
        json!({"callId":"invalid","result":{"error":"unreadable"}}),
    )
    .unwrap();
    assert_eq!(
        tool_output::read_page(&store, "p", "t", "invalid", 0)["code"],
        "RESULT_READ_FAILED"
    );
    store
        .db
        .lock()
        .unwrap()
        .execute_batch("DROP TABLE agent_events")
        .unwrap();
    assert_eq!(
        tool_output::read_page(&store, "p", "t", "any", 0)["code"],
        "RESULT_READ_FAILED"
    );
    assert_eq!(
        agent_artifacts::read_page(&store, "p", &json!({"id":"child","turnId":"turn"}))["code"],
        "AGENT_RESULT_READ_FAILED"
    );
    let id = format!("media-{}", "a".repeat(64));
    assert_eq!(
        media_retention::read(&store.db.lock().unwrap(), "p", &id, &profile())["code"],
        "MEDIA_READ_FAILED"
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

struct CatalogFailure(CancellationToken);
impl Host for CatalogFailure {
    fn token(&self) -> &CancellationToken {
        &self.0
    }
    fn definitions(&self) -> Result<Vec<ToolDefinition>, String> {
        Err("catalog read failed".into())
    }
    fn parallel_safe(&self, _: &ToolCall) -> bool {
        false
    }
    fn record(&self, _: &str, _: Value) -> Result<(), String> {
        Ok(())
    }
    async fn execute(&self, _: &ToolCall) -> Value {
        panic!("catalog failure executed a tool")
    }
}

#[tokio::test]
async fn catalog_failure_stops_before_model_or_tools() {
    let host = CatalogFailure(CancellationToken::new());
    let model = TestModel::default();
    let error = run(
        &model,
        &profile(),
        &host,
        session::Session::new(vec![Message::user("work")]),
        false,
        "",
    )
    .await
    .unwrap_err();
    assert!(error.contains("catalog read failed"));
    assert!(model.requests.lock().unwrap().is_empty());
    assert!(
        scheduler::execute(
            &host,
            &mut session::Session::new(vec![]),
            &[call("call", "write", json!({}))]
        )
        .await
        .unwrap_err()
        .contains("catalog read failed")
    );
}
