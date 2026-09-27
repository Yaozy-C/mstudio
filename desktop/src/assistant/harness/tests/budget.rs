use super::*;
use rig_core::message::{ToolCall, ToolFunction};
use serde_json::json;
#[test]
fn compaction_never_cuts_a_tool_call_from_its_result() {
    let tool = ToolCall::from_wire(
        "call-1",
        ToolFunction {
            name: "read".into(),
            arguments: json!({}),
        },
    );
    let messages = vec![
        Message::System {
            content: "rules".into(),
        },
        Message::user("first"),
        Message::Assistant {
            id: None,
            content: vec![AssistantContent::text("done")],
        },
        Message::Assistant {
            id: None,
            content: vec![AssistantContent::ToolCall(tool.clone())],
        },
        super::super::session::result_message(&tool, &json!({"ok":true})),
        Message::user("current"),
        Message::Assistant {
            id: None,
            content: vec![AssistantContent::text("reply")],
        },
    ];
    assert_eq!(compact_end(&messages, 100_000, 0), Some(5));
    assert_eq!(
        compact_end(&messages, tokens(&messages[1]) + tokens(&messages[2]), 0),
        Some(3)
    );
}
#[test]
fn transport_bytes_are_not_text_tokens() {
    let image = Message::User {
        content: vec![
            UserContent::text("look"),
            UserContent::image_url(
                format!("data:image/jpeg;base64,{}", "a".repeat(500_000)),
                None,
                None,
            ),
        ],
    };
    assert!(tokens(&image) < 5000);
    assert!(tokens(&Message::user("x".repeat(200_000))) > 40_000);
    assert_eq!(text_tokens("中文中文"), 1);
    assert_eq!(text_tokens("😀😀"), 1);
}

#[test]
fn latest_batch_is_neither_compacted_nor_pruned_before_consumption() {
    use crate::assistant::harness::session;
    let call = ToolCall::from_wire(
        "new",
        ToolFunction {
            name: "read".into(),
            arguments: json!({}),
        },
    );
    let mut messages = vec![
        Message::user("request"),
        Message::Assistant {
            id: None,
            content: vec![AssistantContent::ToolCall(call.clone())],
        },
    ];
    for _ in 0..8 {
        messages.push(session::result_message(
            &call,
            &json!({"pixels":"x".repeat(9000)}),
        ));
    }
    assert_eq!(compact_end(&messages, usize::MAX, 0), None);
    // The boundary also covers a snapshot injected after the batch.
    messages.push(Message::user("snapshot"));
    assert_eq!(compact_end(&messages, usize::MAX, 0), None);
}
