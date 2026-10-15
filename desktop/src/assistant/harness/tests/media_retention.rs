use super::super::session;
use super::support::call;
use rig_core::message::{AssistantContent, ImageMediaType, Message, UserContent};
use serde_json::json;

#[test]
fn fresh_image_batch_is_protected_and_only_requested_old_occurrences_are_omitted() {
    use super::support::TestHost;
    let host = TestHost::default();
    let calls: Vec<_> = (0..8)
        .map(|i| call(&format!("frame-{i}"), "mstudio_read_image", json!({})))
        .collect();
    let mut messages = vec![
        Message::user("review frames"),
        Message::Assistant(rig_core::message::AssistantMessage::new(
            calls
                .iter()
                .cloned()
                .map(AssistantContent::ToolCall)
                .collect(),
        )),
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
    assert_eq!(
        super::super::media_retention::offload_oldest_images(&mut session, &host, 3).unwrap(),
        0
    );
    assert_eq!(
        serde_json::to_string(&session.messages)
            .unwrap()
            .matches("aGVsbG8=")
            .count(),
        8
    );
    session.messages.push(Message::Assistant(
        rig_core::message::AssistantMessage::new(vec![AssistantContent::text(
            "reviewed all eight",
        )]),
    ));
    session.messages.push(Message::user("next step"));
    assert_eq!(
        super::super::media_retention::offload_oldest_images(&mut session, &host, 3).unwrap(),
        3
    );
    assert_eq!(
        serde_json::to_string(&session.messages)
            .unwrap()
            .matches("aGVsbG8=")
            .count(),
        5
    );
}

#[test]
fn random_and_noncanonical_offload_references_are_rejected_without_legacy_lookup() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    for id in [
        "image-1792081577843405000-18",
        "image-1",
        "image-AAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAAA",
    ] {
        assert_eq!(
            super::super::stored_image::read(&db, "p", id)["code"],
            "INVALID_ARGS"
        );
    }
    for id in ["media-1792081577843405000-18", "media-1"] {
        assert_eq!(
            super::super::media_retention::read(&db, "p", id, &super::support::profile())["code"],
            "INVALID_ARGS"
        );
    }
}
