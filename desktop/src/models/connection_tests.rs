use super::Model;
use super::commands::{check_connection, listed_model_matches};
use crate::assistant::config::Profile;
use serde_json::json;

#[test]
fn gemini_resource_ids_match_in_both_model_list_protocols() {
    let mut profile = Profile {
        endpoint: "https://generativelanguage.googleapis.com/v1beta/openai".into(),
        model: "gemini-3.7-flash".into(),
        adapter: "openai-compatible".into(),
        ..Default::default()
    };
    assert!(listed_model_matches(
        &profile,
        &json!({"id":"models/gemini-3.7-flash"})
    ));
    assert!(!listed_model_matches(
        &profile,
        &json!({"id":"models/gemini-3.8-flash"})
    ));
    profile.endpoint = "https://gemini-proxy.example".into();
    profile.adapter = "gemini-native".into();
    assert!(listed_model_matches(
        &profile,
        &json!({"name":"models/gemini-3.7-flash"})
    ));
    profile.model = "models/gemini-3.7-flash".into();
    assert!(listed_model_matches(
        &profile,
        &json!({"name":"models/gemini-3.7-flash"})
    ));
    assert!(!listed_model_matches(
        &profile,
        &json!({"displayName":"Gemini 3.7 Flash"})
    ));
}

#[test]
fn non_google_namespaces_are_not_silently_removed() {
    let mut profile = Profile {
        endpoint: "https://api.example/v1".into(),
        model: "demo".into(),
        adapter: "openai-compatible".into(),
        ..Default::default()
    };
    assert!(!listed_model_matches(
        &profile,
        &json!({"id":"models/demo"})
    ));
    assert!(listed_model_matches(&profile, &json!({"id":"demo"})));
    profile.endpoint = "https://generativelanguage.googleapis.com.example/v1".into();
    assert!(!listed_model_matches(
        &profile,
        &json!({"id":"models/demo"})
    ));
    profile.model = "models/demo".into();
    assert!(listed_model_matches(&profile, &json!({"id":"models/demo"})));
}

#[tokio::test]
async fn native_connection_check_accepts_google_resource_name_response() {
    use std::io::{Read, Write};
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", server.local_addr().unwrap());
    let task = std::thread::spawn(move || {
        let (mut stream, _) = server.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut request = Vec::new();
        loop {
            let mut chunk = [0; 1024];
            let n = stream.read(&mut chunk).unwrap();
            assert!(n > 0);
            request.extend_from_slice(&chunk[..n]);
            if request.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }
        let request = String::from_utf8_lossy(&request).to_lowercase();
        assert!(request.starts_with("get /v1beta/models "));
        assert!(request.contains("x-goog-api-key: fixture-key"));
        let body = r#"{"models":[{"name":"models/gemini-3.7-flash"}]}"#;
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
    });
    let model = Model {
        id: "fixture".into(),
        name: "Gemini".into(),
        connection_id: None,
        has_key: true,
        profile: Profile {
            endpoint,
            adapter: "gemini-native".into(),
            model: "gemini-3.7-flash".into(),
            ..Default::default()
        },
    };
    assert!(
        check_connection(&model, "fixture-key")
            .await
            .unwrap()
            .contains("已找到所选模型")
    );
    task.join().unwrap();
}
