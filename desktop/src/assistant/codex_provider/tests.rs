use super::*;
#[tokio::test]
#[ignore = "uses local ChatGPT account for a short text completion"]
async fn live_codex_text() {
    let rows = crate::models::codex_connection::model_list().await.unwrap();
    let row = rows
        .iter()
        .find(|r| r["isDefault"] == true)
        .unwrap_or(&rows[0]);
    let model = CodexModel::new(row["id"].as_str().unwrap().into());
    let result = model
        .completion(
            model
                .completion_request("Reply with exactly MSTUDIO_OK. Do not use tools.")
                .build(),
        )
        .await
        .unwrap();
    assert!(
        serde_json::to_string(&result.choice)
            .unwrap()
            .contains("MSTUDIO_OK")
    );
}
#[tokio::test]
#[ignore = "uses local ChatGPT account to verify two tools within one native turn"]
async fn live_codex_tool_round_trip() {
    use rig_core::{
        completion::ToolDefinition,
        message::{AssistantContent, Message},
    };
    let rows = crate::models::codex_connection::model_list().await.unwrap();
    let row = rows
        .iter()
        .find(|r| r["isDefault"] == true)
        .unwrap_or(&rows[0]);
    let model = CodexModel::new(row["id"].as_str().unwrap().into());
    let mut request = model.completion_request("Call mstudio_test_echo with value=hello. Wait for its result, then call mstudio_test_echo again with value exactly equal to that result. After the second result, reply with the second result verbatim. Do not use any other tools.")
            .tools(vec![ToolDefinition { name: "mstudio_test_echo".into(), description: "Return the test value".into(), parameters: json!({"type":"object","properties":{"value":{"type":"string"}},"required":["value"],"additionalProperties":false}) }]).build();
    let first = model.completion(request.clone()).await.unwrap();
    let call = first
        .choice
        .iter()
        .find_map(|c| {
            if let AssistantContent::ToolCall(c) = c {
                Some(c)
            } else {
                None
            }
        })
        .expect("expected dynamic tool call");
    assert_eq!(call.function.name, "mstudio_test_echo");
    let native = {
        let waiting = model.waiting.lock().unwrap();
        let run = waiting.as_ref().unwrap();
        (run.thread.clone(), run.turn.clone())
    };
    let result = Message::tool_result(call.id.clone(), "mstudio_test_echo", "MSTUDIO_NEXT");
    request.chat_history.push(Message::Assistant {
        id: None,
        content: first.choice,
    });
    request.chat_history.push(result);
    let second = model.completion(request.clone()).await.unwrap();
    assert!(
        second.usage.input_tokens > 0,
        "Codex must report actual usage after the first tool result"
    );
    assert!(
        second.raw["tokenUsage"]["modelContextWindow"]
            .as_u64()
            .unwrap()
            > 0
    );
    assert_eq!(
        second.raw["contextMessageCount"],
        request.chat_history.len() - 1
    );
    let call = second
        .choice
        .iter()
        .find_map(|p| match p {
            AssistantContent::ToolCall(c) => Some(c),
            _ => None,
        })
        .expect("second tool call");
    assert_eq!(call.function.arguments["value"], "MSTUDIO_NEXT");
    {
        let waiting = model.waiting.lock().unwrap();
        let run = waiting.as_ref().unwrap();
        assert_eq!((&run.thread, &run.turn), (&native.0, &native.1));
    }
    let result = Message::tool_result(call.id.clone(), "mstudio_test_echo", "MSTUDIO_TOOL_OK");
    request.chat_history.push(Message::Assistant {
        id: None,
        content: second.choice,
    });
    request.chat_history.push(result);
    let second = model.completion(request).await.unwrap();
    assert!(second.usage.input_tokens > 0);
    assert!(
        second.raw["tokenUsage"]["last"]["totalTokens"]
            .as_u64()
            .unwrap()
            < second.raw["tokenUsage"]["total"]["totalTokens"]
                .as_u64()
                .unwrap()
    );
    println!(
        "Codex usage verified: billed segment={}, last context={}, window={}",
        second.usage.total_tokens,
        second.raw["tokenUsage"]["last"]["totalTokens"],
        second.raw["tokenUsage"]["modelContextWindow"]
    );
    assert!(
        serde_json::to_string(&second.choice)
            .unwrap()
            .contains("MSTUDIO_TOOL_OK")
    );
}
#[tokio::test]
#[ignore = "uses local ChatGPT account to verify a synthetic image input"]
async fn live_codex_sees_native_image() {
    use rig_core::message::{ImageMediaType, Message, UserContent};
    let rows = crate::models::codex_connection::model_list().await.unwrap();
    let row = rows
        .iter()
        .find(|r| r["isDefault"] == true)
        .unwrap_or(&rows[0]);
    let model = CodexModel::new(row["id"].as_str().unwrap().into());
    let mut request = model.completion_request("Identify the dominant color of the attached image. Reply with one English color word only. Do not use tools.").build();
    request.chat_history.push(Message::User { content: vec![UserContent::image_base64("iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAIAAAAlC+aJAAAAb0lEQVR4nO3PAQkAAAyEwO9feoshgnABdLep8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3IPanc8OLDQitxAAAAAElFTkSuQmCC", Some(ImageMediaType::PNG), None)] });
    let response = model.completion(request).await.unwrap();
    assert!(
        serde_json::to_string(&response.choice)
            .unwrap()
            .to_lowercase()
            .contains("red")
    );
}
#[test]
fn only_registered_dynamic_tools_reach_the_harness() {
    let mut event = json!({"params":{"tool":"inspect_project","callId":"call-1","arguments":{}}});
    assert!(tool_frame(&event, &["inspect_project".into()]).is_ok());
    assert!(tool_frame(&event, &[]).is_err());
    event["params"]["arguments"] = json!("invalid");
    assert!(tool_frame(&event, &["inspect_project".into()]).is_err());
}

#[test]
fn readable_reasoning_notifications_become_stream_parts() {
    for (method, index) in [
        ("item/reasoning/summaryTextDelta", "summaryIndex"),
        ("item/reasoning/textDelta", "contentIndex"),
    ] {
        let mut params = json!({"itemId":"r1","delta":"Check the reference", "encryptedContent":"never-display"});
        params[index] = json!(0);
        let part = output::reasoning_delta(method, &params).unwrap();
        let rig_core::streaming::RawStreamingChoice::ReasoningDelta { reasoning, .. } = part else {
            panic!("expected a reasoning delta")
        };
        assert_eq!(reasoning, "Check the reference");
    }
    assert!(output::reasoning_delta("item/agentMessage/delta", &json!({})).is_none());
    assert!(
        output::reasoning_delta(
            "item/reasoning/textDelta",
            &json!({"itemId":"r1","contentIndex":0,"encryptedContent":"secret"})
        )
        .is_none()
    );
}
