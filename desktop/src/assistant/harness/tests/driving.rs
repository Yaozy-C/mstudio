use super::{
    super::{Host, run, session::Session},
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
    assert!(result.unwrap_err().contains("已停止"));
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

#[test]
fn meter_anchors_usage_and_counts_signed_growth_without_double_output() {
    use super::super::budget;
    let host = TestHost::default();
    let route = profile();
    let mut session = Session::new(vec![
        Message::user("中文".repeat(100)),
        Message::assistant("answer"),
    ]);
    let raw = budget::raw_pressure(&session, &host);
    budget::anchor(&mut session, &route, &host, 2000, 500);
    assert_eq!(budget::pressure(&session, &route, &host), 2500);
    session.messages.push(Message::user("new"));
    assert_eq!(
        budget::pressure(&session, &route, &host),
        2500 + budget::raw_pressure(&session, &host) - raw
    );
    let anchor = session.usage_anchor.clone();
    session
        .replace_range(&host, 0, 1, Message::user("摘要"))
        .unwrap();
    assert_eq!(session.usage_anchor, anchor);
    assert!(budget::pressure(&session, &route, &host) < 2500);
    let mut changed = route.clone();
    changed.model = "different".into();
    assert_eq!(
        budget::pressure(&session, &changed, &host),
        budget::raw_pressure(&session, &host)
    );
}

#[test]
fn meter_does_not_scale_down_heuristic_when_usage_is_smaller() {
    use super::super::budget;
    let host = TestHost::default();
    let mut session = Session::new(vec![Message::user("verbose".repeat(2000))]);
    budget::anchor(&mut session, &profile(), &host, 2, 1);
    assert_eq!(
        budget::pressure(&session, &profile(), &host),
        budget::raw_pressure(&session, &host)
    );
}

#[tokio::test]
async fn ordinary_answer_ends_without_business_receipt_gate() {
    let host = TestHost::default();
    let model = TestModel::default();
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![AssistantContent::text("新创意")])));
    assert_eq!(
        run(
            &model,
            &profile(),
            &host,
            Session::new(vec![Message::user("新想一个创意")]),
            false,
            ""
        )
        .await
        .unwrap(),
        "新创意"
    );
    assert_eq!(model.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn saved_write_does_not_implicitly_finish_the_agent() {
    let host = TestHost::default();
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Ok(reply(vec![AssistantContent::ToolCall(call(
            "save",
            "write",
            json!({}),
        ))])),
        Ok(reply(vec![AssistantContent::ToolCall(call(
            "verify",
            "read",
            json!({}),
        ))])),
        Ok(reply(vec![AssistantContent::text("已保存并检查")])),
    ]);
    let answer = run(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::user("保存脚本")]),
        false,
        "",
    )
    .await
    .unwrap();
    assert_eq!(answer, "已保存并检查");
    assert_eq!(model.requests.lock().unwrap().len(), 3);
    assert_eq!(
        *host.trace.lock().unwrap(),
        vec!["start:save", "end:save", "start:verify", "end:verify"]
    );
}

#[tokio::test]
async fn unsplittable_high_estimate_does_not_invent_provider_overflow() {
    let host = TestHost::default();
    let model = TestModel::default();
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![AssistantContent::text("完成")])));
    let mut route = profile();
    route.context_window = Some(8192);
    let answer = run(
        &model,
        &route,
        &host,
        Session::new(vec![Message::user("长资料".repeat(12000))]),
        false,
        "",
    )
    .await
    .unwrap();
    assert_eq!(answer, "完成");
    assert_eq!(model.requests.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn pressure_summary_failure_continues_original_request() {
    let host = TestHost::default();
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Err(CompletionError::ResponseError("summary unavailable".into())),
        Ok(reply(vec![AssistantContent::text("完成")])),
    ]);
    let mut route = profile();
    route.context_window = Some(8192);
    let mut messages = vec![Message::System {
        content: "rules".into(),
    }];
    for _ in 0..4 {
        messages.push(Message::user("旧资料".repeat(2000)));
        messages.push(Message::assistant("旧回答".repeat(2000)));
    }
    messages.push(Message::user("继续"));
    assert_eq!(
        run(&model, &route, &host, Session::new(messages), false, "")
            .await
            .unwrap(),
        "完成"
    );
    assert!(
        host.events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, _)| kind == "compaction/warning")
    );
}

#[tokio::test]
async fn provider_overflow_without_reduction_is_not_retried() {
    let host = TestHost::default();
    let model = TestModel::default();
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Err(CompletionError::HttpError(
            http_client::Error::InvalidStatusCodeWithMessage(
                400.try_into().unwrap(),
                json!({"error":{"message":"maximum context length exceeded"}}).to_string(),
            ),
        )));
    assert!(
        run(
            &model,
            &profile(),
            &host,
            Session::new(vec![Message::user("large attachment")]),
            false,
            ""
        )
        .await
        .unwrap_err()
        .contains("报告上下文")
    );
    assert_eq!(model.requests.lock().unwrap().len(), 1);
}

#[test]
fn cache_usage_is_adapter_aware_and_request_header_is_fixed_at_dispatch() {
    use super::super::budget;
    let host = TestHost::default();
    let mut route = profile();
    let usage = rig_core::completion::Usage {
        input_tokens: 1000,
        output_tokens: 100,
        cached_input_tokens: 700,
        cache_creation_input_tokens: 200,
        ..Default::default()
    };
    assert_eq!(budget::usage_total(&route, &usage), 1100);
    route.adapter = "anthropic-native".into();
    assert_eq!(budget::usage_total(&route, &usage), 2000);
    let mut session = Session::new(vec![
        Message::user("request"),
        Message::assistant("response"),
    ]);
    let mut dispatched = budget::header(&route, &host.definitions());
    dispatched["tools"] = json!([]);
    budget::anchor_request(&mut session, dispatched, 10000);
    // A tool catalog change during the request must not calibrate the new catalog.
    assert_eq!(
        budget::pressure(&session, &route, &host),
        budget::raw_pressure(&session, &host)
    );
}

#[tokio::test]
async fn refusal_has_explicit_stop_reason_and_does_not_execute_tools() {
    let host = TestHost::default();
    let model = TestModel::default();
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![AssistantContent::ToolCall(call(
            "rejected",
            "write",
            json!({}),
        ))])
        .with_finish_reason(rig_core::completion::FinishReason::ContentFilter)));
    let result = run(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::user("request")]),
        false,
        "",
    )
    .await;
    assert!(result.unwrap_err().contains("拒绝"));
    assert!(host.trace.lock().unwrap().is_empty());
    assert!(
        host.events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, value)| kind == "model/stop" && value["stopReason"] == "refusal")
    );
}
