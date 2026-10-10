use super::super::thinking::*;
use super::support::TestHost;
use rig_core::message::{Reasoning, ReasoningContent};

#[test]
fn live_deltas_are_replaced_by_the_complete_block_and_saved_once() {
    let host = TestHost::default();
    let mut thinking = Thinking::new();
    thinking.update(&host, "part", "先", false).unwrap();
    thinking.update(&host, "part", "检查", false).unwrap();
    thinking.update(&host, "part", "先检查参考", true).unwrap();
    thinking.finish(&host).unwrap();
    let events = host.events.lock().unwrap();
    assert_eq!(events[1].1["text"], "先检查");
    assert_eq!(events[2].1["text"], "先检查参考");
    assert_eq!(events[3].0, "assistant/reasoning");
    assert_eq!(events[3].1["text"], "先检查参考");
    assert_eq!(events[0].1["id"], events[3].1["id"]);
}

#[test]
fn signatures_encrypted_and_redacted_content_never_enter_display_text() {
    let reasoning = Reasoning {
        id: Some("private-handle".into()),
        content: vec![
            ReasoningContent::Text {
                text: "可见文字".into(),
                signature: Some("secret-signature".into()),
            },
            ReasoningContent::Encrypted("ciphertext".into()),
            ReasoningContent::Redacted {
                data: "private-data".into(),
            },
            ReasoningContent::Summary("摘要".into()),
        ],
    };
    assert_eq!(readable(&reasoning), "可见文字\n\n摘要");
}
