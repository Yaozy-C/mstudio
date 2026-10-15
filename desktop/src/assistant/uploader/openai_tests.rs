use super::{
    transport_tests::{Captured, Step, server},
    *,
};
use crate::assistant::model_io::CompletionModel;
use rig_core::{
    completion::CompletionRequest,
    message::{ImageMediaType, Message, UserContent},
};
use serde_json::json;

#[tokio::test]
async fn openai_sdk_uploads_with_vision_and_reuses_file_id() {
    let uploaded: Step = (
        200,
        Box::new(|request: &Captured| {
            assert!(request.head.starts_with("POST /v1/files "));
            let body = String::from_utf8_lossy(&request.body);
            assert!(body.contains("name=\"purpose\"\r\n\r\nvision"));
            let filename = body
                .split("filename=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap();
            json!({"id":"file-openai-test","object":"file","bytes":5,"created_at":now(),"filename":filename,"purpose":"vision","expires_at":now()+LIFETIME})
        }),
    );
    let accepted = || -> Step {
        (
            200,
            Box::new(|request| {
                assert!(request.head.starts_with("POST /v1/responses "));
                let body: Value = serde_json::from_slice(&request.body).unwrap();
                assert!(body.to_string().contains("file-openai-test"));
                assert!(!body.to_string().contains("base64"));
                json!({"id":"resp_test","object":"response","created_at":0,"status":"completed","model":"gpt-5",
            "output":[{"id":"msg_test","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"read","annotations":[]}]}],
            "usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}})
            }),
        )
    };
    let (root, server) = server(vec![uploaded, accepted(), accepted()]);
    let root = format!("{root}/v1");
    let uploader =
        Uploader::new(Arc::new(backends::openai::OpenAi), &root, "test-key", None).unwrap();
    let http = rig_core::http_client::DynHttpClient::new(FileTransport::new(
        rig_reqwest::ReqwestClient::from(rig_http::Client::new()),
        uploader,
    ));
    let model = rig_core::providers::openai::OpenAIConfig::new("test-key")
        .with_base_url(&root)
        .connect(http)
        .responses("gpt-5")
        .erase();
    let mut request = CompletionRequest::new("");
    request.chat_history = vec![Message::User {
        content: vec![UserContent::image_base64(
            "aGVsbG8=",
            Some(ImageMediaType::PNG),
            None,
        )],
    }];
    for _ in 0..2 {
        model.completion(request.clone()).await.unwrap();
    }
    assert_eq!(server.join().unwrap().len(), 3);
}

#[test]
fn cache_is_isolated_between_backends_even_with_identical_endpoint_and_key() {
    let a = Uploader::new(
        Arc::new(backends::openai::OpenAi),
        "https://same.test",
        "key",
        None,
    )
    .unwrap();
    let b = Uploader::new(
        Arc::new(backends::deepseek::DeepSeek),
        "https://same.test",
        "key",
        None,
    )
    .unwrap();
    assert_ne!(a.scope, b.scope);
}
