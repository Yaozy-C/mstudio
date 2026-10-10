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

#[tokio::test]
async fn final_answer_excludes_progress_but_keeps_the_exact_model_transcript() {
    let host = TestHost::default();
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Ok(reply(vec![
            AssistantContent::text("正在核对"),
            AssistantContent::ToolCall(call("read", "read", json!({}))),
        ])),
        Ok(reply(vec![AssistantContent::text(
            "已找到原始提示词，结论如下。",
        )])),
    ]);
    let answer = run(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::user("诊断")]),
        false,
        "",
    )
    .await
    .unwrap();
    assert_eq!(answer, "已找到原始提示词，结论如下。");
    assert!(
        model.requests.lock().unwrap()[1]
            .chat_history
            .iter()
            .any(|m| serde_json::to_string(m).unwrap().contains("正在核对"))
    );
    assert!(
        host.events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, v)| kind == "assistant/partial" && v["delta"] == "正在核对")
    );
}

#[tokio::test]
async fn commentary_is_folded_and_final_text_replaces_live_progress() {
    use rig_core::message::{AdditionalParams, Text};
    let host = TestHost::default();
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Ok(reply(vec![
            AssistantContent::text("读取一次"),
            AssistantContent::ToolCall(call("read", "read", json!({}))),
        ])),
        Ok(reply(vec![
            AssistantContent::Text(Text {
                text: "准备保存".into(),
                additional_params: AdditionalParams::from_entries([("phase", json!("commentary"))]),
            }),
            AssistantContent::Text(Text {
                text: "分镜已保存".into(),
                additional_params: AdditionalParams::from_entries([(
                    "phase",
                    json!("final_answer"),
                )]),
            }),
        ])),
    ]);
    let answer = run(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::user("写分镜")]),
        false,
        "",
    )
    .await
    .unwrap();
    assert_eq!(answer, "分镜已保存");
    let events = host.events.lock().unwrap();
    let progress: Vec<_> = events
        .iter()
        .filter(|(k, _)| k == "assistant/progress")
        .map(|(_, v)| v["text"].as_str().unwrap())
        .collect();
    assert_eq!(progress, ["读取一次", "准备保存"]);
    assert_eq!(
        events
            .iter()
            .rev()
            .find(|(k, _)| k == "assistant/partial")
            .unwrap()
            .1["text"],
        "分镜已保存"
    );
    assert!(
        events
            .iter()
            .any(|(k, v)| k == "assistant/partial" && v["text"] == "")
    );
}

#[tokio::test]
async fn reasoning_and_progress_survive_each_tool_step_in_order() {
    use rig_core::message::Reasoning;
    let host = TestHost::default();
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Ok(reply(vec![
            AssistantContent::Reasoning(Reasoning::new("先检查")),
            AssistantContent::text("读取参考"),
            AssistantContent::ToolCall(call("read", "read", json!({}))),
        ])),
        Ok(reply(vec![
            AssistantContent::Reasoning(Reasoning::new("再核对")),
            AssistantContent::text("检查完成"),
        ])),
    ]);
    let answer = run(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::user("检查")]),
        false,
        "",
    )
    .await
    .unwrap();
    assert_eq!(answer, "检查完成");
    let events = host.events.lock().unwrap();
    let visible: Vec<_> = events
        .iter()
        .filter(|(kind, _)| {
            matches!(
                kind.as_str(),
                "assistant/reasoning" | "assistant/progress" | "tool/call"
            )
        })
        .map(|(kind, v)| (kind.as_str(), v["text"].as_str().unwrap_or("tool")))
        .collect();
    assert_eq!(
        visible,
        [
            ("assistant/reasoning", "先检查"),
            ("assistant/progress", "读取参考"),
            ("tool/call", "tool"),
            ("assistant/reasoning", "再核对")
        ]
    );
}

#[tokio::test]
async fn gemini_thinking_models_request_readable_summaries_without_changing_budget() {
    let host = TestHost::default();
    let model = TestModel::default();
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![AssistantContent::text("完成")])));
    let mut route = profile();
    route.adapter = "gemini-native".into();
    route.model = "gemini-3.1-flash".into();
    run(
        &model,
        &route,
        &host,
        Session::new(vec![Message::user("检查")]),
        false,
        "",
    )
    .await
    .unwrap();
    assert_eq!(
        model.requests.lock().unwrap()[0].additional_params,
        Some(json!({"generationConfig":{"thinkingConfig":{"includeThoughts":true}}}))
    );
}
