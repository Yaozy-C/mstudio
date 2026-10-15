use super::provider;
use crate::app_error::AppError;
use serde_json::{Value, json};
use std::io::{Read, Write};

fn completion(
    body: &str,
    status: u16,
    content_type: &str,
    broken_body: bool,
) -> anyhow::Result<String> {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let endpoint = format!("http://{}", listener.local_addr().unwrap());
    let body = body.to_owned();
    let content_type = content_type.to_owned();
    let worker = std::thread::spawn(move || {
        let (mut socket, _) = listener.accept().unwrap();
        socket
            .set_read_timeout(Some(std::time::Duration::from_secs(5)))
            .unwrap();
        let mut bytes = Vec::new();
        loop {
            let mut chunk = [0; 4096];
            let size = socket.read(&mut chunk).unwrap();
            assert_ne!(size, 0);
            bytes.extend_from_slice(&chunk[..size]);
            let Some(end) = bytes.windows(4).position(|w| w == b"\r\n\r\n") else {
                continue;
            };
            let headers = String::from_utf8_lossy(&bytes[..end]);
            let size = headers
                .lines()
                .find_map(|line| {
                    line.to_lowercase()
                        .strip_prefix("content-length: ")
                        .and_then(|v| v.parse::<usize>().ok())
                })
                .unwrap();
            if bytes.len() < end + 4 + size {
                continue;
            }
            let request: Value = serde_json::from_slice(&bytes[end + 4..end + 4 + size]).unwrap();
            assert_eq!(request["stream"], true);
            assert!(request.get("tools").is_none());
            break;
        }
        // Split every UTF-8 character and SSE line across HTTP chunks.
        write!(socket, "HTTP/1.1 {status} Test\r\nContent-Type: {content_type}\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n").unwrap();
        let mut response = Vec::new();
        for byte in body.bytes() {
            response.extend_from_slice(b"1\r\n");
            response.push(byte);
            response.extend_from_slice(b"\r\n");
        }
        if !broken_body {
            response.extend_from_slice(b"0\r\n\r\n");
        }
        socket.write_all(&response).unwrap();
    });
    let model = crate::models::Model {
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
    let result = tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(provider::complete(
            &model,
            "",
            "contract",
            "current file",
            &Default::default(),
        ));
    worker.join().unwrap();
    result
}

fn event(delta: Value, reason: Value) -> String {
    format!(
        "data: {}\r\n\r\n",
        json!({"choices":[{"delta":delta,"finish_reason":reason}]})
    )
}

#[test]
fn fragmented_stream_ignores_keepalive_and_reasoning_and_accepts_only_final_content() {
    let expected = r#"{"message":"已创建","code":"export function createShot() {}"}"#;
    let body = format!(
        ": keep-alive\r\n\r\n{}{}{}data: [DONE]\r\n\r\n",
        event(
            json!({"reasoning_content":"private reasoning"}),
            Value::Null
        ),
        event(json!({"content":expected}), Value::Null),
        event(json!({}), json!("stop")),
    );
    assert_eq!(
        completion(&body, 200, "text/event-stream; charset=utf-8", false).unwrap(),
        expected
    );
}

#[test]
fn stream_rejects_partial_content_truncation_and_tool_calls() {
    let partial = event(json!({"content":"{\"message\":\"partial"}), Value::Null);
    for ending in ["", "data: [DONE]\n\n"] {
        let error = completion(
            &format!("{partial}{ending}"),
            200,
            "text/event-stream",
            false,
        )
        .unwrap_err();
        assert!(error.to_string().contains("接收中断"));
    }
    for (delta, reason, expected) in [
        (json!({}), json!("length"), "被截断"),
        (
            json!({"tool_calls":[{"id":"call"}]}),
            Value::Null,
            "不执行工具调用",
        ),
        (json!({}), json!("content_filter"), "未完成预演回复"),
    ] {
        let error = completion(
            &format!("{partial}{}", event(delta, reason)),
            200,
            "text/event-stream",
            false,
        )
        .unwrap_err();
        assert!(error.to_string().contains(expected));
    }
}

#[test]
fn broken_http_body_is_a_network_failure_with_actionable_diagnostics() {
    let error = completion(
        &event(json!({"content":"partial"}), Value::Null),
        200,
        "text/event-stream",
        true,
    )
    .unwrap_err();
    let issue = AppError::from_error(&error, "PREVIS_CHAT_FAILED", "previs_chat");
    assert_eq!(issue.code, "NETWORK_ERROR");
    assert_eq!(issue.details, "服务连接中断");
    assert!(issue.retryable);
}

#[test]
fn stream_service_errors_and_non_streaming_fallback_remain_supported() {
    assert!(
        completion(
            "data: {\"error\":{\"message\":\"private provider details\"}}\n\n",
            200,
            "text/event-stream",
            false
        )
        .unwrap_err()
        .to_string()
        .contains("流式错误")
    );
    let error = completion(
        r#"{"error":{"message":"Insufficient balance"}}"#,
        402,
        "application/json",
        false,
    )
    .unwrap_err();
    let issue = AppError::from_error(&error, "PREVIS_CHAT_FAILED", "previs_chat");
    assert_eq!(issue.code, "QUOTA_EXCEEDED");
    assert_eq!(issue.http_status, Some(402));
    assert_eq!(issue.details, "Insufficient balance");
    assert_eq!(
        completion(
            "\n\n{\"choices\":[{\"message\":{\"content\":\"hello\"},\"finish_reason\":\"stop\"}]}",
            200,
            "application/json",
            false
        )
        .unwrap(),
        "hello"
    );
}
