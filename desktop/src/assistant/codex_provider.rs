//! Codex app-server transport. Mstudio's harness still owns tool execution and history.
use super::model_io::{CompletionModel, Frame, ModelStream};
use crate::model_adapters::codex_rpc::Rpc;
use rig_core::{
    completion::{CompletionRequest, CompletionResponse, FinishReason},
    error::ProviderError,
    message::{ToolCall, ToolFunction},
};
use serde_json::{Value, json};
use std::time::Duration;

mod connection;
#[cfg(all(test, unix))]
mod continuation_tests;
mod output;
#[cfg(test)]
mod tests;
mod usage;
use connection::Connection;
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct CodexModel {
    model: String,
    waiting: Arc<Mutex<Option<Connection>>>,
}
impl CodexModel {
    pub fn new(model: String) -> Self {
        Self {
            model,
            waiting: Arc::default(),
        }
    }
}
fn failure(error: impl std::fmt::Display) -> ProviderError {
    ProviderError::Provider(format!("Codex: {error}"))
}
#[allow(clippy::result_large_err)] // Rig owns the provider error type at this boundary.
fn tool_frame(event: &Value, names: &[String]) -> Result<Frame, ProviderError> {
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
    Ok(Frame::ToolCall(ToolCall::from_wire(
        id,
        ToolFunction::new(
            name.try_into().map_err(failure)?,
            params["arguments"].clone(),
        ),
    )))
}
impl CompletionModel for CodexModel {
    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        super::model_io::collect(self.stream(request).await?).await
    }
    async fn stream(&self, request: CompletionRequest) -> Result<ModelStream, ProviderError> {
        request.validate_message_content().map_err(failure)?;
        let waiting = self.waiting.lock().unwrap().take();
        let connection = if let Some(mut connection) = waiting {
            if let Some(result) = connection.continuation(&request) {
                let result = output::tool_response(result).map_err(failure)?;
                connection
                    .rpc
                    .send(json!({"id":connection.pending["id"],"result":result}))
                    .await
                    .map_err(failure)?;
                connection
                    .usage
                    .continue_after_tool(request.chat_history.len());
                connection.request = request;
                connection.pending = Value::Null;
                connection
            } else {
                // Context compaction, new input or changed tools starts a new native turn.
                connection.rpc.stop().await;
                Connection::start(&self.model, request).await?
            }
        } else {
            Connection::start(&self.model, request).await?
        };
        let owner = self.waiting.clone();
        let frames = futures::stream::try_unfold(
            (Some(connection), None::<CompletionResponse>, owner),
            |(mut connection, terminal, owner)| async move {
                if let Some(terminal) = terminal {
                    return Ok(Some((Frame::Done(Box::new(terminal)), (None, None, owner))));
                }
                let Some(run) = connection.as_mut() else {
                    return Ok(None);
                };
                loop {
                    let event = tokio::time::timeout(Duration::from_secs(240), run.rpc.next())
                        .await
                        .map_err(failure)?
                        .map_err(failure)?;
                    let params = &event["params"];
                    // Ignore notifications for unrelated native threads/turns.
                    if !run.accepts(params) {
                        continue;
                    }
                    match event["method"].as_str() {
                        Some("thread/tokenUsage/updated") => {
                            run.usage.observe(&params["tokenUsage"]);
                        }
                        Some("item/started") if params["item"]["type"] == "agentMessage" => {
                            if let Some(frame) = output::text_start(&params["item"]) {
                                return Ok(Some((frame, (connection, None, owner))));
                            }
                        }
                        Some("item/agentMessage/delta") => {
                            if let Some(text) = params["delta"].as_str() {
                                return Ok(Some((
                                    Frame::Text(text.into()),
                                    (connection, None, owner),
                                )));
                            }
                        }
                        Some(
                            method @ ("item/reasoning/summaryTextDelta"
                            | "item/reasoning/textDelta"),
                        ) => {
                            if let Some(frame) = output::reasoning_delta(method, params) {
                                return Ok(Some((frame, (connection, None, owner))));
                            }
                        }
                        Some("item/tool/call") => {
                            let names = run
                                .request
                                .tools
                                .iter()
                                .map(|t| t.name.to_string())
                                .collect::<Vec<_>>();
                            let frame = tool_frame(&event, &names)?;
                            if event.get("id").is_none() {
                                return Err(failure("Codex 工具请求缺少响应 ID"));
                            }
                            run.pending = event;
                            let terminal = run.usage.finish(FinishReason::ToolCalls);
                            // Keep the native turn waiting; the harness executes and journals the tool.
                            *owner.lock().unwrap() = connection.take();
                            return Ok(Some((frame, (None, Some(terminal), owner))));
                        }
                        Some("turn/completed") => {
                            run.rpc.stop().await;
                            if params["turn"]["status"] != "completed" {
                                return Err(failure(
                                    params["turn"]["error"]["message"]
                                        .as_str()
                                        .unwrap_or("Codex 对话未完成"),
                                ));
                            }
                            return Ok(Some((
                                Frame::Done(Box::new(run.usage.finish(FinishReason::Stop))),
                                (None, None, owner),
                            )));
                        }
                        _ => run.rpc.decline(&event).await.map_err(failure)?,
                    }
                }
            },
        );
        Ok(super::model_stream::assemble(Box::pin(frames)))
    }
}
