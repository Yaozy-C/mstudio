//! Model boundary shared by HTTP providers and the local Codex transport.
use futures::{StreamExt, stream::BoxStream};
use rig_core::{
    DynModel,
    completion::{CompletionRequest, CompletionResponse},
    error::ProviderError,
    message::{AssistantContent, ToolCall},
    operation::Completion,
    streaming::{Item, StreamEvent},
};

pub enum Frame {
    TextStart {
        id: String,
        phase: Option<String>,
    },
    Text(String),
    Reasoning {
        id: String,
        text: String,
        replace: bool,
    },
    ToolCall(ToolCall),
    Done(Box<CompletionResponse>),
}
pub type ModelStream = BoxStream<'static, Result<Frame, ProviderError>>;

pub trait CompletionModel: Sync {
    fn completion(
        &self,
        request: CompletionRequest,
    ) -> impl Future<Output = Result<CompletionResponse, ProviderError>> + Send;
    fn stream(
        &self,
        request: CompletionRequest,
    ) -> impl Future<Output = Result<ModelStream, ProviderError>> + Send;
}

impl CompletionModel for DynModel<Completion> {
    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, ProviderError> {
        self.call(request).await
    }
    async fn stream(&self, request: CompletionRequest) -> Result<ModelStream, ProviderError> {
        let stream = DynModel::stream(self, request)?;
        Ok(Box::pin(futures::stream::try_unfold(
            Some(stream),
            |state| async move {
                let Some(mut stream) = state else {
                    return Ok(None);
                };
                while let Some(item) = stream.next().await {
                    let frame = match item? {
                        Item::Event(StreamEvent::Text { text, .. }) => Some(Frame::Text(text)),
                        Item::Event(StreamEvent::Reasoning { part, text }) => {
                            Some(Frame::Reasoning {
                                id: part.index().to_string(),
                                text,
                                replace: false,
                            })
                        }
                        Item::Event(StreamEvent::End {
                            part,
                            content: AssistantContent::Reasoning(reasoning),
                        }) => Some(Frame::Reasoning {
                            id: part.index().to_string(),
                            text: reasoning.text,
                            replace: true,
                        }),
                        _ => None,
                    };
                    if let Some(frame) = frame {
                        return Ok(Some((frame, Some(stream))));
                    }
                }
                Ok(Some((Frame::Done(Box::new(stream.finish().await?)), None)))
            },
        )))
    }
}

pub async fn collect(mut stream: ModelStream) -> Result<CompletionResponse, ProviderError> {
    while let Some(frame) = stream.next().await {
        if let Frame::Done(response) = frame? {
            return Ok(*response);
        }
    }
    Err(ProviderError::Truncated)
}
