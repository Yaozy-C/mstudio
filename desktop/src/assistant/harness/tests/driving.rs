use super::{
    super::{run, session::Session},
    support::*,
};
use rig_core::{
    completion::CompletionError,
    http_client,
    message::{AssistantContent, Message},
};
use serde_json::json;
#[tokio::test]
async fn pressure_compacts_old_history_before_the_next_request() {
    let host = TestHost::default();
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Ok(reply(vec![AssistantContent::text("旧任务摘要")])),
        Ok(reply(vec![AssistantContent::text("继续完成")])),
    ]);
    let mut messages = vec![Message::System {
        content: "rules".into(),
    }];
    for _ in 0..4 {
        messages.push(Message::user("旧问题".repeat(2000)));
        messages.push(Message::Assistant {
            id: None,
            content: vec![AssistantContent::text("旧回答".repeat(2000))],
        });
    }
    messages.push(Message::user("当前问题"));
    let mut route = profile();
    route.context_window = Some(8192);
    assert_eq!(
        run(&model, &route, &host, Session::new(messages), false, "")
            .await
            .unwrap(),
        "继续完成"
    );
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    assert!(
        serde_json::to_string(&requests[1].chat_history)
            .unwrap()
            .contains("旧任务摘要")
    );
    assert!(
        host.events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, _)| kind == "session/compaction")
    );
}
#[tokio::test]
async fn provider_context_overflow_compacts_and_retries_once() {
    let host = TestHost::default();
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Err(CompletionError::HttpError(
            http_client::Error::InvalidStatusCodeWithMessage(
                400.try_into().unwrap(),
                json!({"error":{"message":"maximum context length exceeded"}}).to_string(),
            ),
        )),
        Ok(reply(vec![AssistantContent::text("历史摘要")])),
        Ok(reply(vec![AssistantContent::text("完成")])),
    ]);
    let mut messages = vec![Message::System {
        content: "rules".into(),
    }];
    for _ in 0..4 {
        messages.push(Message::user("旧问题".repeat(200)));
        messages.push(Message::Assistant {
            id: None,
            content: vec![AssistantContent::text("旧回答".repeat(200))],
        });
    }
    messages.push(Message::user("当前问题"));
    let mut route = profile();
    route.context_window = Some(16384);
    assert_eq!(
        run(&model, &route, &host, Session::new(messages), false, "")
            .await
            .unwrap(),
        "完成"
    );
    assert_eq!(model.requests.lock().unwrap().len(), 3);
    assert!(
        host.events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, _)| kind == "request/context-overflow")
    );
}
#[tokio::test]
async fn retry_reuses_exact_request_and_does_not_reexecute_committed_write() {
    let host = TestHost::default();
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Ok(reply(vec![AssistantContent::ToolCall(call(
            "edit-1",
            "write",
            json!({}),
        ))])),
        Err(CompletionError::HttpError(
            http_client::Error::InvalidStatusCode(503.try_into().unwrap()),
        )),
        Ok(reply(vec![AssistantContent::text("完成")])),
    ]);
    assert_eq!(
        run(
            &model,
            &profile(),
            &host,
            Session::new(vec![Message::user("do it")]),
            false,
            ""
        )
        .await
        .unwrap(),
        "完成"
    );
    assert_eq!(
        *host.trace.lock().unwrap(),
        vec!["start:edit-1", "end:edit-1"]
    );
    let requests = model.requests.lock().unwrap();
    assert_eq!(
        serde_json::to_value(&requests[1]).unwrap(),
        serde_json::to_value(&requests[2]).unwrap()
    );
    assert!(
        host.events
            .lock()
            .unwrap()
            .iter()
            .any(|(k, _)| k == "request/retry")
    );
}
#[tokio::test]
async fn cancellation_preserves_visible_prefix_and_does_not_retry_stream() {
    let host = TestHost::default();
    let model = TestModel {
        partial_then_stall: true,
        ..Default::default()
    };
    let cancel = async {
        loop {
            if host
                .events
                .lock()
                .unwrap()
                .iter()
                .any(|(k, _)| k == "assistant/partial")
            {
                host.token.cancel();
                break;
            }
            tokio::task::yield_now().await;
        }
    };
    let route = profile();
    let run = run(
        &model,
        &route,
        &host,
        Session::new(vec![Message::user("do it")]),
        true,
        "",
    );
    let (result, ()) = tokio::join!(run, cancel);
    let error: serde_json::Value = serde_json::from_str(&result.unwrap_err()).unwrap();
    assert_eq!(error["code"], "CHAT_STOPPED");
    assert_eq!(error["outcome"], "cancelled");
    assert_eq!(model.requests.lock().unwrap().len(), 1);
    let events = host.events.lock().unwrap();
    assert!(
        events
            .iter()
            .any(|(k, v)| k == "assistant/partial" && v["text"] == "已输出的文字")
    );
}
#[path = "driving_streaming.rs"]
mod streaming;
