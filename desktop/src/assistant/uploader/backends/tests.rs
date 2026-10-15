use super::*;
use serde_json::json;

#[test]
fn select_upload_capabilities_by_platform_and_protocol() {
    let mut profile = Profile::default();
    for (endpoint, adapter, expected) in [
        (
            "https://api.deepseek.com",
            "openai-responses",
            Some("deepseek"),
        ),
        (
            "https://api.deepseek.com",
            "openai-compatible",
            Some("deepseek"),
        ),
        (
            "https://api.deepseek.com/anthropic",
            "anthropic-native",
            Some("deepseek"),
        ),
        (
            "https://api.openai.com/v1",
            "openai-responses",
            Some("openai"),
        ),
        ("https://api.openai.com/v1", "openai-compatible", None),
        ("https://api.openai.com/v1", "codex", None),
        (
            "https://api.deepseek.com.example.org",
            "openai-responses",
            None,
        ),
        ("http://api.deepseek.com", "openai-responses", None),
        ("https://api.openai.com:8443/v1", "openai-responses", None),
        ("http://127.0.0.1:1234", "openai-responses", None),
    ] {
        profile.endpoint = endpoint.into();
        profile.adapter = adapter.into();
        assert_eq!(select(&profile).map(|b| b.name()), expected);
    }
}

#[test]
fn upload_source_is_generic_and_preserves_response_image_options() {
    let pdf = Source::new("application/pdf", b"document".to_vec()).unwrap();
    assert_eq!(openai::OpenAi.purpose(&pdf), "user_data");
    assert!(deepseek::DeepSeek.validate(&pdf).is_ok());
    let png = Source::new("image/png", b"pixels".to_vec()).unwrap();
    assert_eq!(openai::OpenAi.purpose(&png), "vision");
    assert_eq!(deepseek::DeepSeek.purpose(&png), "user_data");
    for backend in [
        Arc::new(openai::OpenAi) as Arc<dyn Backend>,
        Arc::new(deepseek::DeepSeek),
    ] {
        let mut part =
            json!({"type":"input_image","image_url":"data:image/png;base64,YQ==","detail":"high"});
        backend.project(&mut part, "file-test").unwrap();
        assert_eq!(
            part,
            json!({"type":"input_image","file_id":"file-test","detail":"high"})
        );
    }
    let mut chat = json!({"type":"image_url","image_url":{"url":"data:image/png;base64,YQ=="}});
    assert!(openai::OpenAi.project(&mut chat, "file-test").is_err());
    assert!(Source::new("image/png\r\nInjected: yes", vec![1]).is_err());
    assert!(Source::new("image/png", vec![]).is_err());
}
