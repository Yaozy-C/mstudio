use super::*;

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
    let result = crate::assistant::harness::driver::run_until(
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
