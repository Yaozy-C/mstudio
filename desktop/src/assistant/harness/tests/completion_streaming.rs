use super::*;

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
