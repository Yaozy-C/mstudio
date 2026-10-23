use super::{
    super::{
        Host, budget, image_budget, media_retention, run,
        session::{self, Session},
    },
    support::*,
};
use rig_core::{
    completion::ToolDefinition,
    message::{
        AssistantContent, AssistantMessage, DocumentSourceKind, Image, Message, ToolCall,
        ToolResultContent, UserContent,
    },
};
use serde_json::{Value, json};

fn image(data: &str) -> UserContent {
    UserContent::image_base64(data, None, None)
}

#[test]
fn inline_budget_counts_occurrences_and_evicts_only_the_oldest_excess() {
    let host = TestHost::default();
    let mut state = Session::new(vec![
        Message::User {
            content: vec![image("aaaa"), image("bbbb"), image("bbbb")],
        },
        Message::assistant("reviewed"),
        Message::User {
            content: vec![image("cccc")],
        },
    ]);
    let assistant = state.messages[1].clone();
    assert_eq!(image_budget::fit_limit(&mut state, &host, 16).unwrap(), 0);
    assert_eq!(image_budget::fit_limit(&mut state, &host, 12).unwrap(), 1);
    let text = serde_json::to_string(&state.messages).unwrap();
    assert!(!text.contains("aaaa"));
    assert_eq!(text.matches("bbbb").count(), 2);
    assert!(text.contains("cccc") && text.contains("saved reference image-"));
    assert_eq!(state.messages[1], assistant);
    assert_eq!(image_budget::fit_limit(&mut state, &host, 12).unwrap(), 0);
    assert_eq!(host.events.lock().unwrap().len(), 1);
}

#[test]
fn oversized_unseen_batch_and_cancellation_never_discard_evidence() {
    let host = TestHost::default();
    let mut state = Session::new(vec![
        Message::User {
            content: vec![image("aaaa")],
        },
        Message::assistant("reviewed"),
        Message::User {
            content: vec![image("bbbb"), image("cccc")],
        },
    ]);
    let original = state.messages.clone();
    assert!(
        image_budget::fit_limit(&mut state, &host, 4)
            .unwrap_err()
            .contains("新增图片")
    );
    assert_eq!(state.messages, original);
    host.token.cancel();
    assert!(image_budget::fit_limit(&mut state, &host, 8).is_err());
    assert_eq!(state.messages, original);
    assert!(host.events.lock().unwrap().is_empty());
}

#[test]
fn budget_handles_nested_tool_images_raw_bytes_and_reference_urls() {
    let host = TestHost::default();
    let read = call("read", "mstudio_read_image", json!({}));
    let result = session::result_message(
        &read,
        &json!({"imageId":"asset","__offloadedImage":Image {
            data: DocumentSourceKind::Raw(vec![0; 4]), ..Default::default()
        }}),
    );
    let mut state = Session::new(vec![
        result,
        Message::User {
            content: vec![UserContent::image_url(
                "data:image/png;base64,YQ==",
                None,
                None,
            )],
        },
        Message::assistant("reviewed"),
        Message::User {
            content: vec![UserContent::image_url(
                "https://example.test/a.png",
                None,
                None,
            )],
        },
    ]);
    // Four raw bytes occupy eight base64 bytes, plus four data-URL bytes.
    assert_eq!(image_budget::fit_limit(&mut state, &host, 12).unwrap(), 0);
    assert_eq!(image_budget::fit_limit(&mut state, &host, 4).unwrap(), 1);
    assert_eq!(image_budget::images(&state.messages).len(), 2);
    assert!(
        serde_json::to_string(&state.messages)
            .unwrap()
            .contains("asset")
    );
}

#[test]
fn budget_never_omits_assistant_outputs() {
    let host = TestHost::default();
    let UserContent::Image(output) = image("output") else {
        unreachable!()
    };
    let mut state = Session::new(vec![
        Message::user("request"),
        Message::Assistant(AssistantMessage::new(vec![AssistantContent::Image(output)])),
        Message::assistant("continue"),
        Message::user("continue"),
    ]);
    let original = state.messages.clone();
    assert_eq!(
        media_retention::offload_oldest_media(&mut state, &host, 1).unwrap(),
        0
    );
    assert_eq!(state.messages, original);
}

#[tokio::test]
async fn context_pressure_releases_only_enough_old_media_without_summarizing() {
    let host = TestHost::default();
    let model = TestModel::default();
    let mut route = profile();
    route.context_window = Some(262_144);
    let mut state = Session::new(vec![
        Message::User {
            content: vec![image("aaaa"), image("bbbb"), image("cccc")],
        },
        Message::assistant("reviewed"),
        Message::User {
            content: vec![image("dddd")],
        },
    ]);
    budget::anchor(
        &mut state,
        &route,
        &host,
        route.input_budget().unwrap() as u64 + 400,
        0,
    );
    assert!(
        budget::recover(&model, &route, &host, &mut state, false)
            .await
            .unwrap()
    );
    assert!(!budget::over_budget(&state, &route, &host).unwrap());
    assert_eq!(image_budget::images(&state.messages).len(), 3);
    assert!(model.requests.lock().unwrap().is_empty());
}

struct ImageHost(TestHost);
impl Host for ImageHost {
    fn token(&self) -> &tokio_util::sync::CancellationToken {
        &self.0.token
    }
    fn definitions(&self) -> Result<Vec<ToolDefinition>, String> {
        Ok({
            vec![ToolDefinition {
                name: "mstudio_read_image".try_into().unwrap(),
                description: "read pixels".into(),
                parameters: json!({"type":"object","properties":{"assetId":{"type":"string"}},"required":["assetId"]}),
            }]
        })
    }
    fn parallel_safe(&self, _: &ToolCall) -> bool {
        false
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        self.0.record(kind, value)
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        let UserContent::Image(pixels) =
            image(&format!("pixels-{}", call.function.arguments["assetId"]))
        else {
            unreachable!()
        };
        json!({"ok":true,"imageId":call.function.arguments["assetId"],"__offloadedImage":pixels})
    }
}

#[tokio::test]
async fn eight_product_images_survive_three_batches_in_the_real_driver() {
    let host = ImageHost(TestHost::default());
    let model = TestModel::default();
    for batch in [0..3, 3..6, 6..8] {
        let calls = batch
            .map(|i| {
                AssistantContent::ToolCall(call(
                    &format!("call-{i}"),
                    "mstudio_read_image",
                    json!({"assetId":format!("product-{i}")}),
                ))
            })
            .collect();
        let mut response = reply(calls);
        response.usage = rig_core::completion::Usage::new()
            .input_tokens(32_000)
            .output_tokens(1000)
            .total_tokens(33_000);
        model.responses.lock().unwrap().push_back(Ok(response));
    }
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![AssistantContent::text("concept complete")])));
    assert_eq!(
        run(
            &model,
            &crate::assistant::config::Profile {
                endpoint: "https://generativelanguage.googleapis.com".into(),
                adapter: "gemini-native".into(),
                model: "gemini-4-argon".into(),
                ..Default::default()
            },
            &host,
            Session::new(vec![Message::user("develop a concept")]),
            false,
            ""
        )
        .await
        .unwrap(),
        "concept complete"
    );
    let requests = model.requests.lock().unwrap();
    assert!(
        requests
            .iter()
            .all(|request| request.max_tokens == Some(32_768))
    );
    assert_eq!(
        requests
            .iter()
            .map(|r| image_budget::images(&r.chat_history).len())
            .collect::<Vec<_>>(),
        [0, 3, 6, 8]
    );
    for i in 0..8 {
        let text = serde_json::to_string(&requests[3].chat_history).unwrap();
        assert!(text.contains(&format!("product-{i}")));
    }
    assert!(
        !host
            .0
            .events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, _)| kind == "media/offload" || kind == "compaction/start")
    );
    assert!(requests[3].chat_history.iter().any(|m| matches!(m, Message::User { content } if content.iter().any(|c| matches!(c, UserContent::ToolResult(r) if r.content.iter().any(|p| matches!(p, ToolResultContent::Image(_))))))));
}

#[test]
fn uploader_backends_use_raw_bytes_and_their_own_occurrence_limits() {
    let host = TestHost::default();
    let route = crate::assistant::config::Profile {
        endpoint: "https://api.deepseek.com".into(),
        adapter: "openai-responses".into(),
        ..Default::default()
    };
    let mut state = Session::new(vec![Message::User {
        content: vec![image(&"A".repeat(17 * 1024 * 1024))],
    }]);
    assert_eq!(image_budget::fit(&mut state, &host, &route).unwrap(), 0);
    assert!(image_budget::fit(&mut state, &host, &profile()).is_err());
    let mut state = Session::new(vec![
        Message::User {
            content: vec![image("YQ==")],
        },
        Message::assistant("reviewed"),
        Message::User {
            content: (0..600).map(|_| image("Yg==")).collect(),
        },
    ]);
    assert_eq!(image_budget::fit(&mut state, &host, &route).unwrap(), 1);
    assert_eq!(image_budget::images(&state.messages).len(), 600);
    let openai = crate::assistant::config::Profile {
        endpoint: "https://api.openai.com/v1".into(),
        ..route.clone()
    };
    let mut state = Session::new(vec![Message::User {
        content: (0..601).map(|_| image("YQ==")).collect(),
    }]);
    assert_eq!(image_budget::fit(&mut state, &host, &openai).unwrap(), 0);
    assert!(image_budget::fit(&mut state, &host, &route).is_err());
}
