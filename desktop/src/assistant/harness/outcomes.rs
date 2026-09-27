//! Keep delegation failures visible even when the model omits them from its answer.
use rig_core::message::{Message, ToolResultContent, UserContent};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Outcomes(BTreeMap<String, String>);
impl Outcomes {
    pub fn from_messages(messages: &[Message]) -> Self {
        let mut outcomes = Self::default();
        for message in messages {
            if let Message::User { content } = message {
                for part in content {
                    if let UserContent::ToolResult(result) = part {
                        for item in &result.content {
                            if let ToolResultContent::Text(text) = item
                                && let Ok(value) = serde_json::from_str(&text.text)
                            {
                                outcomes.observe(&value);
                            }
                        }
                    }
                }
            }
        }
        outcomes
    }
    pub fn observe(&mut self, value: &Value) {
        let Some(child) = value["childId"].as_str() else {
            return;
        };
        if value["ok"] == true && value["stopReason"] == "completed" {
            self.0.remove(child);
        } else if value["ok"] == false {
            let reason = match value["stopReason"].as_str() {
                Some("step-limit") => "达到执行轮次上限",
                Some("aborted") => "被中止",
                Some("max-tokens") => "输出被截断",
                _ => "执行异常",
            };
            let saved = if value["applied"] == true {
                "已有修改已保存；"
            } else {
                ""
            };
            self.0.insert(
                child.into(),
                format!(
                    "子任务 {child} {reason}，{saved}该次委派未完整完成，剩余工作与复查结果未确认。"
                ),
            );
        }
    }
    pub fn notice(&self) -> String {
        if self.0.is_empty() {
            return String::new();
        }
        format!(
            "\n\n执行状态说明：\n{}",
            self.0.values().cloned().collect::<Vec<_>>().join("\n")
        )
    }
}
