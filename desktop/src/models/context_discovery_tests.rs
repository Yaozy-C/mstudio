use super::context_commands::discover;
use crate::assistant::config::Profile;
use std::io::{Read, Write};

async fn request(
    status: u16,
    body: &'static str,
    adapter: &str,
) -> anyhow::Result<Option<super::context::Limits>> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let profile = Profile {
        endpoint: format!("http://{}", listener.local_addr().unwrap()),
        model: "uncatalogued".into(),
        adapter: adapter.into(),
        ..Default::default()
    };
    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut buffer = [0; 4096];
        let count = stream.read(&mut buffer).unwrap();
        assert!(String::from_utf8_lossy(&buffer[..count]).starts_with("GET "));
        write!(stream, "HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",body.len()).unwrap();
    });
    let result = discover(&profile, "fixture").await;
    server.join().unwrap();
    result
}
#[tokio::test]
async fn capacity_discovery_does_not_equate_auth_or_bad_json_with_missing_fields() {
    for adapter in ["gemini-native", "openai-compatible"] {
        let error = request(401, r#"{"error":{"message":"key rejected"}}"#, adapter)
            .await
            .err()
            .unwrap();
        assert!(error.to_string().contains("AUTH_REQUIRED"));
        let error = request(200, "invalid JSON", adapter).await.err().unwrap();
        assert!(!error.to_string().contains("AUTH_REQUIRED"));
        let body = if adapter == "gemini-native" {
            r#"{"name":"models/uncatalogued"}"#
        } else {
            r#"{"data":[{"id":"uncatalogued"}]}"#
        };
        assert!(request(200, body, adapter).await.unwrap().is_none());
    }
}
