//! Explicit opt-in uses one synthetic red PNG and removes its uploaded file.
use super::*;
use crate::assistant::model_io::CompletionModel;
use rig_core::{
    completion::CompletionRequest,
    message::{AssistantContent, ImageMediaType, Message, UserContent},
};
use serde_json::json;

#[tokio::test]
#[ignore = "uses MSTUDIO_DEEPSEEK_TEST_DB credentials with a synthetic image and one billed model request"]
async fn live_deepseek_file_upload_reuse_and_vision() {
    let path = std::env::var("MSTUDIO_DEEPSEEK_TEST_DB").expect("explicit test database path");
    let db = Connection::open_with_flags(path, rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY).unwrap();
    let key:String=db.query_row("SELECT api_key FROM service_connections WHERE endpoint='https://api.deepseek.com' AND kind='openai-responses'",[],|r|r.get(0)).unwrap();
    drop(db);
    let files = Uploader::new(
        Arc::new(backends::deepseek::DeepSeek),
        "https://api.deepseek.com",
        &key,
        None,
    )
    .unwrap();
    const RED_PNG: &str = "iVBORw0KGgoAAAANSUhEUgAAAEAAAABACAIAAAAlC+aJAAAAb0lEQVR4nO3PAQkAAAyEwO9feoshgnABdLep8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3I8QUNyPEFDcjxBQ3IPanc8OLDQitxAAAAAElFTkSuQmCC";
    let body=json!({"input":[{"role":"user","content":[{"type":"input_image","image_url":format!("data:image/png;base64,{RED_PNG}")}]}]}).to_string();
    let (first, used) = files.prepare(body.as_bytes()).await.unwrap();
    let reused = files.prepare(body.as_bytes()).await;
    let inner = rig_reqwest::ReqwestClient::from(
        rig_http::Client::builder()
            .redirect(rig_http::redirect::Policy::none())
            .timeout(Duration::from_secs(90))
            .build()
            .unwrap(),
    );
    let http = rig_core::http_client::DynHttpClient::new(FileTransport::new(inner, files.clone()));
    let model = rig_core::providers::openai::OpenAIConfig::new(&key)
        .with_base_url("https://api.deepseek.com")
        .connect(http)
        .responses("deepseek-flash")
        .erase();
    let mut request = CompletionRequest::new("");
    request.chat_history = vec![Message::User {
        content: vec![
            UserContent::text(
                "What is the single dominant color of this image? Reply with one lowercase color word.",
            ),
            UserContent::image_base64(RED_PNG, Some(ImageMediaType::PNG), None),
        ],
    }];
    request.max_tokens = Some(64);
    request.additional_params = Some(json!({"reasoning":{"effort":"none"},"store":false}));
    let response = model.completion(request).await;
    // Clean up the exact file created by this test even if model verification failed.
    let deleted = files
        .client
        .delete(format!("{}/files/{}", files.root, used[0].id))
        .bearer_auth(&key)
        .send()
        .await
        .unwrap();
    assert!(
        deleted.status().is_success(),
        "synthetic file cleanup failed"
    );
    assert_eq!(first, reused.unwrap().0);
    let answer: String = response
        .unwrap()
        .choice
        .iter()
        .filter_map(|part| match part {
            AssistantContent::Text(text) => Some(text.text.as_str()),
            _ => None,
        })
        .collect();
    assert_eq!(answer.trim().to_ascii_lowercase(), "red");
}
