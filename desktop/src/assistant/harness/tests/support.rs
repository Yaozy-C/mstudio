use super::super::Host;
use rig_core::{
    completion::{
        CompletionError, CompletionModel, CompletionRequest, CompletionResponse, ToolDefinition,
    },
    message::{AssistantContent, Message, ToolCall, ToolFunction},
    streaming::{RawStreamingChoice, StreamFinal, StreamingCompletionResponse},
};
use serde_json::{Value, json};
use std::{
    collections::VecDeque,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio_util::sync::CancellationToken;
#[derive(Default)]
pub struct TestHost {
    pub token: CancellationToken,
    pub events: Mutex<Vec<(String, Value)>>,
    pub trace: Mutex<Vec<String>>,
    pub active: AtomicUsize,
    pub peak: AtomicUsize,
    pub inbox: Mutex<Vec<Message>>,
    pub settlement_after_step: Mutex<Option<Message>>,
}
impl Host for TestHost {
    fn token(&self) -> &CancellationToken {
        &self.token
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        self.events.lock().unwrap().push((kind.into(), value));
        if kind == "step/end"
            && let Some(message) = self.settlement_after_step.lock().unwrap().take()
        {
            self.inbox.lock().unwrap().push(message);
        }
        Ok(())
    }
    fn injected(&self) -> Result<Vec<Message>, String> {
        Ok(std::mem::take(&mut *self.inbox.lock().unwrap()))
    }
    fn definitions(&self) -> Vec<ToolDefinition> {
        ["read", "write", "ping"].into_iter().map(|name|ToolDefinition {name:name.into(),description:name.into(),parameters:json!({"type":"object","properties":{"delay":{"type":"integer"},"cancel":{"type":"boolean"}},"additionalProperties":false})}).collect()
    }
    fn parallel_safe(&self, call: &ToolCall) -> bool {
        call.function.name != "write"
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        let n = self.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.peak.fetch_max(n, Ordering::SeqCst);
        if call.function.name == "write" {
            assert_eq!(n, 1, "write crossed parallel barrier");
        }
        self.trace
            .lock()
            .unwrap()
            .push(format!("start:{}", call.id.as_str()));
        if call.function.arguments["cancel"] == true {
            self.token.cancel();
        }
        tokio::time::sleep(std::time::Duration::from_millis(
            call.function.arguments["delay"].as_u64().unwrap_or(1),
        ))
        .await;
        self.trace
            .lock()
            .unwrap()
            .push(format!("end:{}", call.id.as_str()));
        self.active.fetch_sub(1, Ordering::SeqCst);
        json!({"ok":true})
    }
}
pub fn call(id: &str, name: &str, args: Value) -> ToolCall {
    ToolCall::from_wire(
        id,
        ToolFunction {
            name: name.into(),
            arguments: args,
        },
    )
}
pub fn reply(content: Vec<AssistantContent>) -> CompletionResponse {
    CompletionResponse::new(content, Default::default(), "test")
}
pub fn profile() -> crate::assistant::config::Profile {
    crate::assistant::config::Profile {
        endpoint: "http://localhost".into(),
        model: "test".into(),
        adapter: "openai-compatible".into(),
        context_window: None,
        inputs: Default::default(),
    }
}
#[derive(Clone, Default)]
pub struct TestModel {
    pub responses: Arc<Mutex<VecDeque<Result<CompletionResponse, CompletionError>>>>,
    pub requests: Arc<Mutex<Vec<CompletionRequest>>>,
    pub partial_then_stall: bool,
}
impl CompletionModel for TestModel {
    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, CompletionError> {
        self.requests.lock().unwrap().push(request);
        self.responses
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected model request")
    }
    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<StreamingCompletionResponse, CompletionError> {
        use futures::StreamExt;
        self.requests.lock().unwrap().push(request);
        let frames =
            futures::stream::iter([Ok(RawStreamingChoice::Message("已输出的文字".into()))]);
        let rest: rig_core::streaming::StreamingResult = if self.partial_then_stall {
            Box::pin(futures::stream::pending())
        } else {
            Box::pin(futures::stream::iter([Ok(
                RawStreamingChoice::FinalResponse(StreamFinal::new("test", Default::default())),
            )]))
        };
        Ok(StreamingCompletionResponse::stream(
            "test",
            Box::pin(frames.chain(rest)),
        ))
    }
}
