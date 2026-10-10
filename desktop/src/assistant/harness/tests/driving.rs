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
#[tokio::test]
async fn streaming_response_is_committed_as_exact_message_and_returns_visible_text() {
    let host = TestHost::default();
    let model = TestModel::default();
    assert_eq!(
        run(
            &model,
            &profile(),
            &host,
            Session::new(vec![Message::user("do it")]),
            true,
            ""
        )
        .await
        .unwrap(),
        "已输出的文字"
    );
    assert!(
        host.events
            .lock()
            .unwrap()
            .iter()
            .any(|(k, v)| k == "session/message" && v.to_string().contains("已输出的文字"))
    );
}

#[tokio::test]
async fn truncated_response_pairs_calls_without_executing_them() {
    let host = TestHost::default();
    let model = TestModel::default();
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![AssistantContent::ToolCall(call(
            "edit",
            "write",
            json!({}),
        ))])
        .with_finish_reason(rig_core::completion::FinishReason::Length)));
    let result = run(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::user("edit")]),
        false,
        "",
    )
    .await;
    assert!(result.unwrap_err().contains("截断"));
    assert!(host.trace.lock().unwrap().is_empty());
    assert!(
        host.events
            .lock()
            .unwrap()
            .iter()
            .any(|(k, v)| k == "tool/result" && v["result"]["code"] == "MODEL_OUTPUT_INCOMPLETE")
    );
}

#[tokio::test]
async fn duplicate_call_ids_are_rejected_before_any_execution() {
    let host = TestHost::default();
    let model = TestModel::default();
    let tool = AssistantContent::ToolCall(call("duplicate", "write", json!({})));
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![tool.clone(), tool])));
    let result = run(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::user("edit")]),
        false,
        "",
    )
    .await;
    assert!(result.unwrap_err().contains("重复"));
    assert!(host.trace.lock().unwrap().is_empty());
}

#[tokio::test]
async fn delegated_run_respects_inherited_deadline_and_keeps_partial_output() {
    let host = TestHost::default();
    let model = TestModel {
        partial_then_stall: true,
        ..Default::default()
    };
    let result = super::super::driver::run_until(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::user("task")]),
        true,
        "",
        tokio::time::Instant::now() + std::time::Duration::from_millis(10),
    )
    .await;
    assert!(result.unwrap_err().contains("时限"));
    assert_eq!(model.requests.lock().unwrap().len(), 1);
    assert!(
        host.events
            .lock()
            .unwrap()
            .iter()
            .any(|(k, _)| k == "assistant/partial")
    );
}

#[tokio::test]
async fn settlement_arriving_with_final_answer_reaches_parent_before_completion() {
    let host = TestHost::default();
    *host.settlement_after_step.lock().unwrap() = Some(Message::user(
        "Subagent concept settled this turn with status idle. Final message: use the packing concept",
    ));
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Ok(reply(vec![AssistantContent::text(
            "Creative direction is still running",
        )])),
        Ok(reply(vec![AssistantContent::ToolCall(call(
            "save",
            "write",
            json!({}),
        ))])),
        Ok(reply(vec![AssistantContent::text("Script saved")])),
    ]);
    let answer = run(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::user("Create the ad script")]),
        false,
        "",
    )
    .await
    .unwrap();
    assert_eq!(answer, "Script saved");
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert!(
        serde_json::to_string(&requests[1].chat_history)
            .unwrap()
            .contains("use the packing concept")
    );
    assert_eq!(
        host.trace
            .lock()
            .unwrap()
            .iter()
            .filter(|s| s.as_str() == "start:save")
            .count(),
        1
    );
}
