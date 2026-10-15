use super::{
    transport_tests::{server, transport, upload},
    *,
};
use crate::assistant::model_io::CompletionModel;
use rig_core::{
    completion::CompletionRequest,
    message::{
        AssistantContent, AssistantMessage, Message, ToolCall, ToolFunction, ToolResultContent,
        UserContent,
    },
};
use serde_json::json;

#[tokio::test]
async fn real_responses_sdk_preserves_tool_pairing_and_sends_native_file_ids() {
    let (root, server) = server(vec![
        upload("file-api-sdk"),
        (
            200,
            Box::new(|request| {
                assert!(request.head.starts_with("POST /responses "));
                let value: Value = serde_json::from_slice(&request.body).unwrap();
                let text = value.to_string();
                assert!(!text.contains("aGVsbG8=") && !text.contains("base64"));
                assert_eq!(text.matches("file-api-sdk").count(), 2, "{value}");
                assert!(
                    value["input"]
                        .as_array()
                        .unwrap()
                        .iter()
                        .any(|item| item["type"] == "function_call_output"
                            && item["call_id"] == "read-image"
                            && item["output"][1]["file_id"] == "file-api-sdk")
                );
                json!({"id":"resp_test","object":"response","created_at":0,"status":"completed","model":"deepseek-flash",
            "output":[{"id":"msg_test","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"read","annotations":[]}]}],
            "usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}})
            }),
        ),
    ]);
    let http = rig_core::http_client::DynHttpClient::new(transport(&root, None));
    let model = rig_core::providers::openai::OpenAIConfig::new("test-key")
        .with_base_url(&root)
        .connect(http)
        .responses("deepseek-flash")
        .erase();
    let UserContent::Image(pixels) = UserContent::image_base64(
        "aGVsbG8=",
        Some(rig_core::message::ImageMediaType::PNG),
        None,
    ) else {
        unreachable!()
    };
    let call = ToolCall::from_wire(
        "read-image",
        ToolFunction::new("read_image".try_into().unwrap(), json!({})),
    );
    let mut request = CompletionRequest::new("");
    request.tools = vec![rig_core::completion::ToolDefinition {
        name: "read_image".try_into().unwrap(),
        description: "Read pixels".into(),
        parameters: json!({"type":"object","properties":{}}),
    }];
    request.chat_history = vec![
        Message::User {
            content: vec![
                UserContent::text("Review image"),
                UserContent::Image(pixels.clone()),
            ],
        },
        Message::Assistant(AssistantMessage::new(vec![AssistantContent::ToolCall(
            call.clone(),
        )])),
        Message::User {
            content: vec![UserContent::ToolResult(call.result(vec![
                ToolResultContent::text("image"),
                ToolResultContent::Image(pixels),
            ]))],
        },
    ];
    let original = request.chat_history.clone();
    let response = model.completion(request.clone()).await.unwrap();
    assert!(matches!(&response.choice[0],AssistantContent::Text(text) if text.text=="read"));
    assert_eq!(request.chat_history, original);
    assert_eq!(server.join().unwrap().len(), 2);
}

#[tokio::test]
async fn chat_and_anthropic_sdks_keep_uploaded_tool_images() {
    for api in ["chat", "anthropic"] {
        let (root, server) = server(vec![
            upload("file-api-sdk"),
            (
                200,
                Box::new(move |request| {
                    let value: Value = serde_json::from_slice(&request.body).unwrap();
                    assert_eq!(
                        value.to_string().matches("file-api-sdk").count(),
                        2,
                        "{value}"
                    );
                    if api == "anthropic" {
                        assert!(request.head.contains("files-api-2025-04-14"));
                        json!({"id":"msg_test","type":"message","role":"assistant","model":"deepseek-flash","content":[{"type":"text","text":"read"}],"stop_reason":"end_turn","usage":{"input_tokens":1,"output_tokens":1}})
                    } else {
                        json!({"id":"test","object":"chat.completion","created":0,"model":"deepseek-flash","choices":[{"index":0,"message":{"role":"assistant","content":"read"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}})
                    }
                }),
            ),
        ]);
        let http = rig_core::http_client::DynHttpClient::new(transport(&root, None));
        let model = if api == "chat" {
            rig_core::providers::openai::OpenAIConfig::with_key(
                &rig_core::providers::openai::wire::Dialect::gateway("mstudio-compatible", "", ""),
                "test-key",
            )
            .with_base_url(&root)
            .connect(http)
            .chat("deepseek-flash")
            .erase()
        } else {
            rig_core::providers::anthropic::AnthropicConfig::new("test-key")
                .with_base_url(&root)
                .connect(http)
                .completion("deepseek-flash")
                .erase()
        };
        let UserContent::Image(pixels) = UserContent::image_base64(
            "aGVsbG8=",
            Some(rig_core::message::ImageMediaType::PNG),
            None,
        ) else {
            unreachable!()
        };
        let call = ToolCall::from_wire(
            "read-image",
            ToolFunction::new("read_image".try_into().unwrap(), json!({})),
        );
        let mut request = CompletionRequest::new("");
        request.tools = vec![rig_core::completion::ToolDefinition {
            name: "read_image".try_into().unwrap(),
            description: "read pixels".into(),
            parameters: json!({"type":"object","properties":{}}),
        }];
        request.chat_history = vec![
            Message::User {
                content: vec![
                    UserContent::text("Review image"),
                    UserContent::Image(pixels.clone()),
                ],
            },
            Message::Assistant(AssistantMessage::new(vec![AssistantContent::ToolCall(
                call.clone(),
            )])),
            Message::User {
                content: vec![UserContent::ToolResult(
                    call.result(vec![ToolResultContent::Image(pixels)]),
                )],
            },
        ];
        request.max_tokens = Some(64);
        let response = model.completion(request).await.unwrap();
        assert!(matches!(&response.choice[0],AssistantContent::Text(text) if text.text=="read"));
        assert_eq!(server.join().unwrap().len(), 2);
    }
}
