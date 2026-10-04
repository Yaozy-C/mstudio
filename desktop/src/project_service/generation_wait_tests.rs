use super::*;
use crate::projects::write_document;
use futures::poll;

fn fixture() -> Store {
    let root =
        std::env::temp_dir().join(format!("mstudio-generation-wait-{}", mstudio::media::id()));
    let store = Store::open(root).unwrap();
    for id in ["p", "other"] {
        write_document(&store, json!({"id":id,"name":id,"nodes":[],"assets":[],"clips":[],
            "production":{"drafts":{"task":{"key":"task","kind":"image","modelId":"m","status":"READY"}}}}), true).unwrap();
    }
    store
}
fn change(store: &Store, project: &str, status: &str, assets: Value) {
    let mut p = load(&store.db.lock().unwrap(), project).unwrap();
    p["production"]["drafts"]["task"]["status"] = json!(status);
    p["production"]["drafts"]["task"]["resultAssetIds"] = assets;
    write_document(store, p, false).unwrap();
}
fn deadline() -> tokio::time::Instant {
    tokio::time::Instant::now() + std::time::Duration::from_secs(5)
}
fn cleanup(store: Store) {
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn interrupted_wait_restores_current_task_facts_without_resubmitting() {
    use crate::assistant::harness::session;
    use rig_core::message::{AssistantContent, Message, ToolCall, ToolFunction};
    let store = fixture();
    let binding = json!({"model":"test"});
    let call = ToolCall::from_wire(
        "wait",
        ToolFunction {
            name: "mstudio_await_generation".into(),
            arguments: json!({"taskKeys":["task"]}),
        },
    );
    let messages = vec![Message::Assistant {
        id: None,
        content: vec![AssistantContent::ToolCall(call)],
    }];
    session::start(&store, "p", "turn", binding.clone(), &messages).unwrap();
    change(&store, "p", "COMPLETED", json!(["persisted"]));
    let restored = session::restore(&store, "p", "turn", &binding)
        .unwrap()
        .unwrap();
    let result = serde_json::to_string(restored.last().unwrap()).unwrap();
    assert!(result.contains("persisted"));
    assert!(result.contains("interrupted"));
    assert!(!result.contains("EFFECT_UNKNOWN"));
    assert_eq!(
        store
            .db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM jobs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    cleanup(store);
}

#[test]
fn parent_handoff_uses_current_outcome_instead_of_an_old_creation_receipt() {
    use crate::assistant::{generation_context, journal};
    let store = fixture();
    journal::append(&store, "p", "child", "tool/call", json!({"callId":"create","arguments":{"operations":[{"op":"request_generation","id":"shot","mediaKind":"image"}]}})).unwrap();
    journal::append(&store, "p", "child", "tool/result", json!({"callId":"create","result":{"detailOffloaded":true},"value":{"applied":true,"generationTasks":[{"id":"task","status":"AWAITING_CONFIRMATION"}]}})).unwrap();
    change(&store, "p", "COMPLETED", json!(["actual-result"]));
    let result = generation_context::outcome(&store, "p", "child").unwrap();
    assert_eq!(result["generationTasks"][0]["status"], "COMPLETED");
    assert_eq!(
        result["generationTasks"][0]["resultAssetIds"][0],
        "actual-result"
    );
    assert_eq!(
        result["generationTasks"][0]["continuation"]["state"],
        "ready"
    );
    assert!(
        generation_context::outcome(&store, "other", "child").unwrap()["generationTasks"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    cleanup(store);
}

#[tokio::test]
async fn waits_for_committed_results_without_status_polling_or_model_round_trips() {
    let store = fixture();
    let token = CancellationToken::new();
    let keys = vec!["task".into()];
    {
        let future = wait(&store, "p", &keys, &token, deadline());
        tokio::pin!(future);
        assert!(poll!(&mut future).is_pending());
        change(&store, "other", "COMPLETED", json!(["unrelated"]));
        assert!(poll!(&mut future).is_pending());
        change(&store, "p", "IN_PROGRESS", json!([]));
        assert!(poll!(&mut future).is_pending());
        // Remote completion is not an imported asset.
        change(&store, "p", "COMPLETED", json!([]));
        assert!(poll!(&mut future).is_pending());
        change(&store, "p", "COMPLETED", json!(["actual-image"]));
        let result = future.await.unwrap();
        assert_eq!(
            result["generationTasks"][0]["resultAssetIds"],
            json!(["actual-image"])
        );
        assert_eq!(
            result["generationTasks"][0]["continuation"]["state"],
            "ready"
        );
    }
    assert_eq!(
        store
            .db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM jobs", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    cleanup(store);
}

#[tokio::test]
async fn confirmation_returns_immediately_and_restart_reads_durable_results() {
    let store = fixture();
    let keys = vec!["task".into()];
    let token = CancellationToken::new();
    change(&store, "p", "AWAITING_CONFIRMATION", json!([]));
    let result = wait(&store, "p", &keys, &token, deadline()).await.unwrap();
    assert_eq!(
        result["generationTasks"][0]["continuation"]["action"],
        "confirm_generation"
    );
    assert!(result.get("waitEnded").is_none());
    change(&store, "p", "COMPLETED", json!(["saved-result"]));
    let root = store.root.clone();
    drop(store);
    let store = Store::open(root).unwrap();
    let result = wait(&store, "p", &keys, &token, deadline()).await.unwrap();
    assert_eq!(
        result["generationTasks"][0]["resultAssetIds"][0],
        "saved-result"
    );
    cleanup(store);
}

#[tokio::test]
async fn cancellation_and_deadline_end_waiting_without_mutating_jobs() {
    let store = fixture();
    let keys = vec!["task".into()];
    let token = CancellationToken::new();
    {
        let future = wait(&store, "p", &keys, &token, deadline());
        tokio::pin!(future);
        assert!(poll!(&mut future).is_pending());
        token.cancel();
        assert_eq!(future.await.unwrap()["waitEnded"], "cancelled");
    }
    let result = wait(
        &store,
        "p",
        &keys,
        &CancellationToken::new(),
        tokio::time::Instant::now(),
    )
    .await
    .unwrap();
    assert_eq!(result["waitEnded"], "deadline");
    assert_eq!(result["generationTasks"][0]["status"], "READY");
    cleanup(store);
}

#[tokio::test]
async fn any_failed_or_removed_task_releases_the_batch_wait() {
    let store = fixture();
    let keys = vec!["task".into()];
    let token = CancellationToken::new();
    {
        let future = wait(&store, "p", &keys, &token, deadline());
        tokio::pin!(future);
        assert!(poll!(&mut future).is_pending());
        change(&store, "p", "FAILED", json!([]));
        assert_eq!(
            future.await.unwrap()["generationTasks"][0]["status"],
            "FAILED"
        );
    }
    let missing = wait(&store, "other", &["absent".into()], &token, deadline())
        .await
        .unwrap();
    assert_eq!(missing["generationTasks"][0]["status"], "REMOVED");
    assert!(
        wait(&store, "deleted-project", &keys, &token, deadline())
            .await
            .is_err()
    );
    cleanup(store);
}
