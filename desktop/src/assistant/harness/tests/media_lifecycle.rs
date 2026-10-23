use super::{
    super::{
        run,
        session::{self, Session},
    },
    support::*,
};
use rig_core::message::{
    AssistantContent, Audio, Document, DocumentSourceKind, Message, UserContent, Video,
};
use serde_json::json;

fn media() -> Vec<UserContent> {
    let data = || DocumentSourceKind::Base64("bWVkaWEtbGlmZWN5Y2xl".into());
    vec![
        UserContent::image_base64("bWVkaWEtbGlmZWN5Y2xl", None, None),
        UserContent::Audio(Audio {
            data: data(),
            ..Default::default()
        }),
        UserContent::Video(Video {
            data: data(),
            ..Default::default()
        }),
        UserContent::Document(Document {
            data: data(),
            ..Default::default()
        }),
        super::super::context_source::attachment("bWVkaWEtbGlmZWN5Y2xl", "text-file"),
    ]
}

#[test]
fn pressure_can_offload_text_file_body_and_preserve_user_text() {
    let payload = json!([
        {"type":"text","text":"user instructions"},
        {"type":"text","text":"Reference: notes.txt (asset ID: text-file)"},
        {"type":"text","text":"attachment-only-body","attachmentId":"text-file"}
    ]);
    let message = crate::assistant::agent::convert(&json!({"role":"user","content":payload}));
    let mut wire = vec![message.clone()];
    super::super::context_source::wire(&mut wire);
    let text = serde_json::to_string(&wire).unwrap();
    assert!(text.contains("attachment-only-body"));
    assert!(!text.contains("mstudio.attachment"));
    let history = crate::assistant::context::text_only(&payload).to_string();
    assert!(!history.contains("attachment-only-body"));
    assert!(history.contains("user instructions") && history.contains("notes.txt"));
    let mut state = Session::new(vec![message, Message::assistant("processed")]);
    assert_eq!(
        super::super::media_retention::offload_oldest_media(&mut state, &TestHost::default(), 1)
            .unwrap(),
        1
    );
    let text = serde_json::to_string(&state.messages).unwrap();
    assert!(!text.contains("attachment-only-body"));
    assert!(text.contains("user instructions") && text.contains("notes.txt"));
}

#[tokio::test]
async fn attachments_remain_across_model_requests_without_budget_pressure() {
    let host = TestHost::default();
    let model = TestModel::default();
    for index in 0..2 {
        model
            .responses
            .lock()
            .unwrap()
            .push_back(Ok(reply(vec![AssistantContent::ToolCall(call(
                &format!("read-{index}"),
                "read",
                json!({}),
            ))])));
    }
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![AssistantContent::text("done")])));
    run(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::User { content: media() }]),
        false,
        "",
    )
    .await
    .unwrap();
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    assert!(
        serde_json::to_string(&requests[0].chat_history)
            .unwrap()
            .contains("bWVkaWEtbGlmZWN5Y2xl")
    );
    for request in &requests[1..] {
        let text = serde_json::to_string(&request.chat_history).unwrap();
        assert!(text.contains("bWVkaWEtbGlmZWN5Y2xl"));
        assert!(!text.contains("omitted to fit"));
    }
    let events = host.events.lock().unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "media/offload")
            .count(),
        0
    );
    assert!(!events.iter().any(|(kind, _)| kind == "compaction/start"));
}

#[test]
fn budget_omissions_never_remove_unseen_followup_media() {
    let host = TestHost::default();
    let mut state = Session::new(vec![
        Message::User { content: media() },
        Message::assistant("reviewed"),
    ]);
    assert_eq!(
        super::super::media_retention::offload_oldest_media(&mut state, &host, 5).unwrap(),
        5
    );
    state.messages.push(Message::User { content: media() });
    assert_eq!(
        super::super::media_retention::offload_oldest_media(&mut state, &host, 5).unwrap(),
        0
    );
    assert_eq!(
        serde_json::to_string(&state.messages)
            .unwrap()
            .matches("bWVkaWEtbGlmZWN5Y2xl")
            .count(),
        5
    );
}

#[test]
fn offloaded_media_replays_as_references_and_reopens_with_project_and_model_checks() {
    use super::super::Host;
    use crate::{assistant::journal, database::Store};
    struct JournalHost<'a>(&'a Store, tokio_util::sync::CancellationToken);
    impl Host for JournalHost<'_> {
        fn token(&self) -> &tokio_util::sync::CancellationToken {
            &self.1
        }
        fn definitions(&self) -> Result<Vec<rig_core::completion::ToolDefinition>, String> {
            Ok(vec![])
        }
        fn parallel_safe(&self, _: &rig_core::message::ToolCall) -> bool {
            false
        }
        fn record(&self, kind: &str, value: serde_json::Value) -> Result<(), String> {
            journal::append(self.0, "p", "t", kind, value).map_err(|e| e.to_string())
        }
        async fn execute(&self, _: &rig_core::message::ToolCall) -> serde_json::Value {
            json!({})
        }
    }
    let root = std::env::temp_dir().join(format!("media-lifecycle-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','p','{}',0)", [])
        .unwrap();
    let binding = json!({"model":"m"});
    let messages = vec![
        Message::User { content: media() },
        Message::assistant("reviewed"),
    ];
    session::start(&store, "p", "t", binding.clone(), &messages).unwrap();
    let mut state = Session::new(messages);
    let host = JournalHost(&store, Default::default());
    assert_eq!(
        super::super::media_retention::offload_oldest_media(&mut state, &host, 5).unwrap(),
        5
    );
    {
        let db = store.db.lock().unwrap();
        let through = db
            .query_row("SELECT max(seq) FROM agent_events", [], |r| r.get(0))
            .unwrap();
        crate::database::session_checkpoint::fold(&db, "p", "t", through).unwrap();
    }
    let restored = session::restore(&store, "p", "t", &binding)
        .unwrap()
        .unwrap();
    assert_eq!(restored, state.messages);
    let model = crate::assistant::attachment_tests::multimodal();
    let db = store.db.lock().unwrap();
    let raw: String = db
        .query_row(
            "SELECT payload FROM agent_events WHERE kind='media/offload'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let value: serde_json::Value = serde_json::from_str(&raw).unwrap();
    for entry in value["offloads"].as_array().unwrap() {
        let id = entry["id"].as_str().unwrap();
        let result = if id.starts_with("image-") {
            super::super::stored_image::read(&db, "p", id)
        } else {
            assert_eq!(
                super::super::media_retention::read(&db, "other", id, &model)["code"],
                "MEDIA_NOT_FOUND"
            );
            if entry["content"]["type"] != "text" {
                assert_eq!(
                    super::super::media_retention::read(&db, "p", id, &profile())["code"],
                    "MEDIA_UNSUPPORTED"
                );
            }
            super::super::media_retention::read(&db, "p", id, &model)
        };
        assert_eq!(result["ok"], true);
        let receipt =
            session::result_message(&call("reopen", "mstudio_reopen_media", json!({})), &result);
        assert!(
            serde_json::to_string(&receipt)
                .unwrap()
                .contains("bWVkaWEtbGlmZWN5Y2xl")
        );
        let mut reopened = Session::new(vec![receipt, Message::assistant("read again")]);
        assert_eq!(
            super::super::media_retention::offload_oldest_media(
                &mut reopened,
                &TestHost::default(),
                1
            )
            .unwrap(),
            1
        );
        assert!(
            !serde_json::to_string(&reopened.messages)
                .unwrap()
                .contains("bWVkaWEtbGlmZWN5Y2xl")
        );
        assert!(
            serde_json::to_string(&reopened.messages)
                .unwrap()
                .contains(id)
        );
    }
    drop(db);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
