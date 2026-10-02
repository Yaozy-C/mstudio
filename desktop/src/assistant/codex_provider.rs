//! Codex app-server transport. Mstudio's harness still owns tool execution and history.
use crate::model_adapters::codex_rpc::Rpc;
use futures::StreamExt;
use rig_core::{
    completion::{
        CompletionError, CompletionModel, CompletionRequest, CompletionResponse, FinishReason,
    },
    streaming::{
        RawStreamingChoice, RawStreamingToolCall, StreamFinal, StreamingCompletionResponse,
    },
};
use serde_json::{Value, json};
use std::time::Duration;

#[derive(Clone)]
pub struct CodexModel(pub String);
fn failure(error: impl std::fmt::Display) -> CompletionError {
    CompletionError::ResponseError(error.to_string())
}
fn tool_frame(event: &Value, names: &[String]) -> Result<RawStreamingChoice, CompletionError> {
    let params = &event["params"];
    let name = params["tool"]
        .as_str()
        .ok_or_else(|| failure("Codex 工具名称缺失"))?;
    if !names.iter().any(|allowed| allowed == name) || !params["arguments"].is_object() {
        return Err(failure("Codex 请求了未授权的工具"));
    }
    let id = params["callId"]
        .as_str()
        .ok_or_else(|| failure("Codex 工具调用 ID 缺失"))?;
    Ok(RawStreamingChoice::ToolCall(RawStreamingToolCall::new(
        id.to_owned(),
        name.into(),
        params["arguments"].clone(),
    )))
}
impl CompletionModel for CodexModel {
    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, CompletionError> {
        let mut stream = self.stream(request).await?;
        while let Some(frame) = stream.next().await {
            frame?;
        }
        Ok(stream.into())
    }
    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse, CompletionError> {
        request.validate_message_content().map_err(failure)?;
        let names: Vec<_> = request.tools.iter().map(|t| t.name.clone()).collect();
        let tools: Vec<_> = request
            .tools
            .iter()
            .map(|t| json!({"name":t.name,"description":t.description,"inputSchema":t.parameters}))
            .collect();
        let setup = async {
            let mut rpc = Rpc::start_text().await.map_err(failure)?;
            let account = rpc
                .call("account/read", json!({"refreshToken":false}))
                .await
                .map_err(failure)?;
            if account["account"]["type"] != "chatgpt" {
                return Err(failure("请在服务连接中登录 Codex"));
            }
            let started = rpc.call("thread/start", json!({
                "model":self.0, "ephemeral":true, "cwd":std::env::temp_dir(),
                "approvalPolicy":"untrusted", "sandbox":"read-only", "dynamicTools":tools,
                "developerInstructions":format!("You are the conversation model for Mstudio. Continue the provided serialized conversation history. Use only the supplied dynamic tools; Mstudio executes them. Never use built-in tools, shell, filesystem, web, MCP or plugins. Answer the user's latest request. {}", request.preamble.as_deref().unwrap_or("")),
                "config":{"features":{"shell_tool":false,"apply_patch_freeform":false},"web_search":"disabled"}
            })).await.map_err(failure)?;
            let thread = started["thread"]["id"]
                .as_str()
                .ok_or_else(|| failure("Codex 未返回会话"))?;
            let history = serde_json::to_string(&request.chat_history).map_err(failure)?;
            rpc.call("turn/start", json!({"threadId":thread,"input":[{"type":"text","text":history}],"outputSchema":request.output_schema})).await.map_err(failure)?;
            Ok::<_, CompletionError>(rpc)
        };
        let rpc = tokio::time::timeout(Duration::from_secs(40), setup)
            .await
            .map_err(failure)??;
        let frames =
            futures::stream::try_unfold((rpc, 0_u8, names), |(mut rpc, stage, names)| async move {
                if stage == 2 {
                    return Ok(None);
                }
                if stage == 1 {
                    return Ok(Some((
                        RawStreamingChoice::FinalResponse(
                            StreamFinal::new("codex", Default::default())
                                .with_finish_reason(FinishReason::ToolCalls),
                        ),
                        (rpc, 2, names),
                    )));
                }
                loop {
                    let event = tokio::time::timeout(Duration::from_secs(240), rpc.next())
                        .await
                        .map_err(failure)?
                        .map_err(failure)?;
                    match event["method"].as_str() {
                        Some("item/agentMessage/delta") => {
                            if let Some(text) = event["params"]["delta"].as_str() {
                                return Ok(Some((
                                    RawStreamingChoice::Message(text.into()),
                                    (rpc, 0, names),
                                )));
                            }
                        }
                        Some("item/tool/call") => {
                            let frame = tool_frame(&event, &names)?;
                            // The outer harness executes this call with its normal permission checks.
                            // The next request replays history including its tool result in a fresh thread.
                            rpc.stop().await;
                            return Ok(Some((frame, (rpc, 1, names))));
                        }
                        Some("turn/completed") => {
                            rpc.stop().await;
                            if event["params"]["turn"]["status"] != "completed" {
                                return Err(failure("Codex 对话未完成，请重试"));
                            }
                            return Ok(Some((
                                RawStreamingChoice::FinalResponse(
                                    StreamFinal::new("codex", Default::default())
                                        .with_finish_reason(FinishReason::Stop),
                                ),
                                (rpc, 2, names),
                            )));
                        }
                        _ => rpc.decline(&event).await.map_err(failure)?,
                    }
                }
            });
        Ok(StreamingCompletionResponse::stream(
            "codex",
            Box::pin(frames),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "uses local ChatGPT account for a short text completion"]
    async fn live_codex_text() {
        let rows = crate::models::codex_connection::model_list().await.unwrap();
        let row = rows
            .iter()
            .find(|r| r["isDefault"] == true)
            .unwrap_or(&rows[0]);
        let model = CodexModel(row["id"].as_str().unwrap().into());
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
    #[ignore = "uses local ChatGPT account to verify one synthetic tool round trip"]
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
        let model = CodexModel(row["id"].as_str().unwrap().into());
        let mut request = model.completion_request("Call mstudio_test_echo once with value=hello. After receiving its result, reply with the result verbatim. Do not use any other tools.")
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
        let result = Message::tool_result(call.id.clone(), "mstudio_test_echo", "MSTUDIO_TOOL_OK");
        request.chat_history.push(Message::Assistant {
            id: None,
            content: first.choice,
        });
        request.chat_history.push(result);
        let second = model.completion(request).await.unwrap();
        assert!(
            serde_json::to_string(&second.choice)
                .unwrap()
                .contains("MSTUDIO_TOOL_OK")
        );
    }
    #[test]
    fn only_registered_dynamic_tools_reach_the_harness() {
        let mut event =
            json!({"params":{"tool":"inspect_project","callId":"call-1","arguments":{}}});
        assert!(tool_frame(&event, &["inspect_project".into()]).is_ok());
        assert!(tool_frame(&event, &[]).is_err());
        event["params"]["arguments"] = json!("invalid");
        assert!(tool_frame(&event, &["inspect_project".into()]).is_err());
    }
}
