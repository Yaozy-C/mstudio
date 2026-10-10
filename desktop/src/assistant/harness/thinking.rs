//! Only provider-returned readable text reaches the UI; opaque payloads stay private.
use super::Host;
use rig_core::message::{Reasoning, ReasoningContent};
use serde_json::json;

pub fn readable(reasoning: &Reasoning) -> String {
    reasoning
        .content
        .iter()
        .filter_map(|part| match part {
            ReasoningContent::Text { text, .. } | ReasoningContent::Summary(text) => {
                Some(text.as_str())
            }
            _ => None,
        })
        .collect::<Vec<_>>()
        .join("\n\n")
}

pub struct Thinking {
    request_id: String,
    blocks: Vec<(String, String)>,
}
impl Thinking {
    pub fn new() -> Self {
        Self {
            request_id: mstudio::media::id(),
            blocks: vec![],
        }
    }
    pub fn update(
        &mut self,
        host: &impl Host,
        id: &str,
        text: &str,
        replace: bool,
    ) -> Result<(), String> {
        let index = self
            .blocks
            .iter()
            .position(|(key, _)| key == id)
            .unwrap_or_else(|| {
                self.blocks.push((id.into(), String::new()));
                self.blocks.len() - 1
            });
        let block = &mut self.blocks[index].1;
        if replace {
            *block = text.into();
        } else {
            block.push_str(text);
        }
        host.record(
            "assistant/thinking",
            json!({"id":format!("{}:{id}", self.request_id),"text":block}),
        )
    }
    pub fn finish(&self, host: &impl Host) -> Result<(), String> {
        for (id, text) in &self.blocks {
            if !text.trim().is_empty() {
                host.record(
                    "assistant/reasoning",
                    json!({"id":format!("{}:{id}", self.request_id),"text":text}),
                )?;
            }
        }
        Ok(())
    }
}
