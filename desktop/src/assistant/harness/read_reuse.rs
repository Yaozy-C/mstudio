//! Remind on repeated identical successful reads without hiding pixels or stale-state checks.
use rig_core::message::{AssistantContent, Message, ToolCall, ToolResultContent, UserContent};
use serde_json::Value;

pub fn annotate(history: &[Message], call: &ToolCall, value: &Value, message: &mut Message) {
    if !call.function.name.starts_with("mstudio_read_") || value.get("error").is_some() {
        return;
    }
    let Message::User { content } = message else {
        return;
    };
    let [UserContent::ToolResult(current)] = content.as_mut_slice() else {
        return;
    };
    let mut matching = Vec::new();
    let mut count = 0;
    for message in history {
        match message {
            Message::Assistant(assistant) => {
                for part in &assistant.content {
                    if let AssistantContent::ToolCall(previous) = part
                        && previous.function == call.function
                    {
                        matching.push(previous.id.clone());
                    }
                }
            }
            Message::User { content } => {
                for part in content {
                    if let UserContent::ToolResult(previous) = part
                        && matching.contains(&previous.call)
                        && previous.content.starts_with(&current.content)
                    {
                        count += 1;
                    }
                }
            }
            _ => {}
        }
    }
    if count > 0 {
        current.content.push(ToolResultContent::text(format!(
            "Host read-reuse notice: this same request and result already appeared {count} time(s) in the supplied history. The complete result remains available here. Reuse it and proceed to the next missing deliverable; read again only for a specific unresolved detail or changed state."
        )));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::harness::session::result_message;
    use rig_core::message::ToolFunction;
    use serde_json::json;
    #[test]
    fn repeats_keep_results_while_changed_results_and_errors_are_not_flagged() {
        let call = ToolCall::from_wire(
            "one",
            ToolFunction::new(
                "mstudio_read_shots".try_into().unwrap(),
                json!({"ids":["s"]}),
            ),
        );
        let value = json!({"revision":1,"prompt":"saved"});
        let history = vec![
            Message::Assistant(rig_core::message::AssistantMessage::new(vec![
                AssistantContent::ToolCall(call.clone()),
            ])),
            result_message(&call, &value),
        ];
        let mut repeated = result_message(&call, &value);
        annotate(&history, &call, &value, &mut repeated);
        assert!(
            serde_json::to_string(&repeated)
                .unwrap()
                .contains("Host read-reuse notice")
        );
        for changed in [
            json!({"revision":2,"prompt":"changed"}),
            json!({"error":"unavailable"}),
        ] {
            let mut message = result_message(&call, &changed);
            let original = message.clone();
            annotate(&history, &call, &changed, &mut message);
            assert_eq!(message, original);
        }
    }
}
