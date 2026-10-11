//! Assemble local transport events into the same typed response HTTP providers return.
use super::model_io::{Frame, ModelStream};
use futures::StreamExt;
use rig_core::message::{AssistantContent, Reasoning};
use serde_json::json;

pub fn assemble(stream: ModelStream) -> ModelStream {
    Box::pin(futures::stream::try_unfold(
        (
            stream,
            Vec::<AssistantContent>::new(),
            None::<(String, Option<String>)>,
        ),
        |(mut stream, mut content, mut active)| async move {
            let Some(frame) = stream.next().await else {
                return Ok(None);
            };
            let mut frame = frame?;
            match &mut frame {
                Frame::TextStart { id, phase } => {
                    active = Some((id.clone(), phase.clone()));
                    let mut part = AssistantContent::text("");
                    part = part.with_native(json!({"id":id,"phase":phase}));
                    content.push(part);
                }
                Frame::Text(text) => {
                    if active.is_none() {
                        content.push(AssistantContent::text(""));
                        active = Some((String::new(), None));
                    }
                    if let Some(AssistantContent::Text(part)) = content.last_mut() {
                        part.text.push_str(text);
                        let (id, phase) = active.as_ref().unwrap();
                        let updated = AssistantContent::Text(part.clone())
                            .with_native(json!({"id":id,"phase":phase}));
                        *content.last_mut().unwrap() = updated;
                    }
                }
                Frame::Reasoning { text, .. } => {
                    active = None;
                    match content.last_mut() {
                        Some(AssistantContent::Reasoning(reasoning)) => {
                            reasoning.text.push_str(text)
                        }
                        _ => {
                            content.push(AssistantContent::Reasoning(Reasoning::new(text.clone())))
                        }
                    }
                }
                Frame::ToolCall(call) => {
                    active = None;
                    content.push(AssistantContent::ToolCall(call.clone()));
                }
                Frame::Done(response) => response.choice = content.clone(),
            }
            Ok(Some((frame, (stream, content, active))))
        },
    ))
}
