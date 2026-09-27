use super::super::session;
use super::support::call;
use rig_core::message::{AssistantContent, ImageMediaType, Message, UserContent};
use serde_json::json;

#[test]
fn fresh_image_batch_survives_pressure_until_a_model_response() {
    use super::support::TestHost;
    let host = TestHost::default();
    let calls: Vec<_> = (0..8)
        .map(|i| call(&format!("frame-{i}"), "mstudio_read_image", json!({})))
        .collect();
    let mut messages = vec![
        Message::user("review frames"),
        Message::Assistant {
            id: None,
            content: calls
                .iter()
                .cloned()
                .map(AssistantContent::ToolCall)
                .collect(),
        },
    ];
    for call in &calls {
        let image = UserContent::image_base64("aGVsbG8=", Some(ImageMediaType::PNG), None);
        let UserContent::Image(image) = image else {
            unreachable!()
        };
        messages.push(session::result_message(
            call,
            &json!({"imageId":call.id,"__offloadedImage":image}),
        ));
    }
    let mut session = session::Session::new(messages);
    // Injecting a snapshot after the batch must not make unseen images eligible.
    session
        .messages
        .push(Message::user("current project snapshot"));
    assert_eq!(session::offload_old_images(&mut session, &host).unwrap(), 0);
    assert_eq!(
        serde_json::to_string(&session.messages)
            .unwrap()
            .matches("aGVsbG8=")
            .count(),
        8
    );
    session.messages.push(Message::Assistant {
        id: None,
        content: vec![AssistantContent::text("reviewed all eight")],
    });
    session.messages.push(Message::user("next step"));
    assert_eq!(session::offload_old_images(&mut session, &host).unwrap(), 8);
    assert!(
        !serde_json::to_string(&session.messages)
            .unwrap()
            .contains("aGVsbG8=")
    );
}
