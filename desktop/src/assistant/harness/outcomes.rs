//! Keep delegation failures visible even when the model omits them from its answer.
use rig_core::message::{Message, ToolResultContent, UserContent};
use serde_json::Value;
use std::collections::BTreeMap;

#[derive(Default)]
pub struct Outcomes {
    failures: BTreeMap<String, String>,
    pending: BTreeMap<String, Vec<Value>>,
    reminded: std::collections::BTreeSet<String>,
}
impl Outcomes {
    pub fn from_messages(messages: &[Message]) -> Self {
        let mut outcomes = Self::default();
        outcomes.observe_messages(messages);
        outcomes
    }
    pub fn observe_messages(&mut self, messages: &[Message]) {
        for message in messages {
            if let Message::User { content } = message {
                for part in content {
                    if let UserContent::Text(text) = part
                        && text.text.starts_with("Subagent ")
                        && let Some((_, raw)) = text.text.split_once(". Final message: ")
                        && let Ok(mut value) = serde_json::from_str::<Value>(raw)
                        && value.is_object()
                        && value["outputRef"]["id"].is_string()
                    {
                        value["childId"] = value["outputRef"]["id"].clone();
                        self.observe(&value);
                    }
                    if let UserContent::ToolResult(result) = part {
                        for item in &result.content {
                            if let ToolResultContent::Text(text) = item
                                && let Ok(value) = serde_json::from_str(&text.text)
                            {
                                self.observe(&value);
                            }
                        }
                    }
                }
            }
        }
    }
    pub fn observe(&mut self, value: &Value) {
        // A later specialist may finish the missing work. Match actual task
        // receipts across runs so stale incomplete notices do not survive recovery.
        for missing in self.pending.values_mut() {
            missing.retain(|item| {
                !value["generationTasks"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|task| {
                        task["targetId"] == item["targetId"]
                            && task["kind"] == item["kind"]
                            && task["id"].as_str().is_some_and(|id| !id.is_empty())
                    })
            });
        }
        self.pending.retain(|_, missing| !missing.is_empty());
        let Some(child) = value["childId"].as_str() else {
            return;
        };
        if value["delivery"]["complete"] == false {
            let missing: Vec<_> = value["delivery"]["items"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|item| item["created"] == false)
                .cloned()
                .collect();
            if missing.is_empty() {
                self.failures
                    .insert(child.into(), format!("子任务 {child} 的任务交付尚未核实。"));
            } else {
                self.pending.insert(child.into(), missing);
            }
        } else if value["ok"] == true && value["stopReason"] == "completed" {
            self.failures.remove(child);
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
            self.failures.insert(
                child.into(),
                format!(
                    "子任务 {child} {reason}，{saved}该次委派未完整完成，剩余工作与复查结果未确认。"
                ),
            );
        }
    }
    pub fn recovery(&mut self) -> Option<String> {
        if self.pending.is_empty() {
            return None;
        }
        let targets: std::collections::BTreeSet<_> = self
            .pending
            .values()
            .flatten()
            .map(|item| {
                (
                    item["targetId"].as_str().unwrap_or(""),
                    item["kind"].as_str().unwrap_or(""),
                )
            })
            .collect();
        let signature = serde_json::to_string(&targets).ok()?;
        if !self.reminded.insert(signature) {
            return None;
        }
        Some(format!(
            "Host delivery check: {}\nBefore ending, reconcile these missing task receipts. Continue only the remaining authorized work; do not duplicate created tasks, bypass confirmation, or override a hard blocker. If blocked, report the specific missing prerequisite and leave the deliverable explicitly incomplete. Quality risk alone is not a repair prerequisite.",
            self.notice()
        ))
    }
    pub fn notice(&self) -> String {
        if self.failures.is_empty() && self.pending.is_empty() {
            return String::new();
        }
        format!(
            "\n\n执行状态说明：\n{}",
            self.failures
                .values()
                .cloned()
                .chain(self.pending.iter().map(|(child, items)| {
                    format!(
                        "子任务 {child} 的生成任务交付未齐（{}）；已保存提示词不等于已创建任务。",
                        items
                            .iter()
                            .filter_map(|i| i["targetId"].as_str())
                            .collect::<Vec<_>>()
                            .join("、")
                    )
                }))
                .collect::<Vec<_>>()
                .join("\n")
        )
    }
}
