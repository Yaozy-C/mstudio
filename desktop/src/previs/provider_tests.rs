use super::*;
use serde_json::json;

#[test]
fn response_provider_extracts_only_final_text_and_rejects_truncation() {
    assert_eq!(provider::text("gemini-native",&json!({"candidates":[{"finishReason":"STOP","content":{"parts":[{"thought":true,"text":"private reasoning"},{"text":"final JSON"}]}}]})).unwrap(),"final JSON");
    assert!(
        provider::text(
            "openai-compatible",
            &json!({"choices":[{"finish_reason":"length","message":{"content":"partial"}}]})
        )
        .is_err()
    );
    assert!(
        provider::text(
            "openai-compatible",
            &json!({"choices":[{"message":{"content":"text","tool_calls":[{}]}}]})
        )
        .is_err()
    );
}
#[test]
fn plain_provider_makes_one_tool_free_request() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let worker = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        let mut bytes = vec![];
        let mut chunk = [0; 4096];
        loop {
            let n = socket.read(&mut chunk).unwrap();
            bytes.extend_from_slice(&chunk[..n]);
            if let Some(i) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..i]);
                let size = headers
                    .lines()
                    .find_map(|l| {
                        l.to_lowercase()
                            .strip_prefix("content-length: ")
                            .and_then(|s| s.parse::<usize>().ok())
                    })
                    .unwrap();
                if bytes.len() >= i + 4 + size {
                    let body: serde_json::Value =
                        serde_json::from_slice(&bytes[i + 4..i + 4 + size]).unwrap();
                    assert_eq!(body["messages"].as_array().unwrap().len(), 2);
                    assert!(body.get("tools").is_none());
                    assert!(!body.to_string().contains("project_snapshot"));
                    break;
                }
            }
        }
        let body = r#"{"choices":[{"finish_reason":"stop","message":{"content":"hello"}}]}"#;
        write!(
            socket,
            "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
            body.len()
        )
        .unwrap();
    });
    let m = crate::models::Model {
        id: "test".into(),
        name: "Test".into(),
        connection_id: None,
        has_key: false,
        profile: crate::assistant::config::Profile {
            endpoint,
            model: "test".into(),
            adapter: "openai-compatible".into(),
            ..Default::default()
        },
    };
    let rt = tokio::runtime::Runtime::new().unwrap();
    assert_eq!(
        rt.block_on(provider::complete(
            &m,
            "",
            "runtime contract",
            "current file only",
            &Default::default()
        ))
        .unwrap(),
        "hello"
    );
    worker.join().unwrap();
}
