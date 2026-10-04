//! Exercise the real HTTP/SSE adapters through the new host loop, not Rig's agent.chat.
use super::{
    super::{run, session::Session},
    support::*,
};
use crate::assistant::{config::Profile, provider};
use rig_core::message::Message;
use serde_json::{Value, json};
use std::io::{Read, Write};
fn request(server: &std::net::TcpListener) -> (std::net::TcpStream, String, Value) {
    let start = std::time::Instant::now();
    let mut stream = loop {
        match server.accept() {
            Ok((s, _)) => break s,
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                assert!(start.elapsed().as_secs() < 10);
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(e) => panic!("{e}"),
        }
    };
    stream.set_nonblocking(false).unwrap();
    stream
        .set_read_timeout(Some(std::time::Duration::from_secs(5)))
        .unwrap();
    let mut bytes = Vec::new();
    let mut buffer = [0; 4096];
    loop {
        let n = stream.read(&mut buffer).unwrap();
        assert!(n > 0);
        bytes.extend_from_slice(&buffer[..n]);
        if let Some(pos) = bytes.windows(4).position(|v| v == b"\r\n\r\n") {
            let headers = String::from_utf8_lossy(&bytes[..pos]).to_lowercase();
            let len: usize = headers
                .lines()
                .find_map(|l| {
                    l.strip_prefix("content-length:")
                        .and_then(|v| v.trim().parse().ok())
                })
                .unwrap();
            if bytes.len() >= pos + 4 + len {
                return (
                    stream,
                    headers,
                    serde_json::from_slice(&bytes[pos + 4..pos + 4 + len]).unwrap(),
                );
            }
        }
    }
}
fn respond(mut stream: std::net::TcpStream, frames: &[Value], done: bool) {
    let body = frames
        .iter()
        .map(|v| format!("data: {v}\n\n"))
        .collect::<String>()
        + if done { "data: [DONE]\n\n" } else { "" };
    write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
}
#[tokio::test]
async fn gemini_stream_keeps_signatures_and_native_call_results_across_steps() {
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    server.set_nonblocking(true).unwrap();
    let profile = Profile {
        endpoint: format!("http://{}", server.local_addr().unwrap()),
        adapter: "gemini-native".into(),
        ..profile()
    };
    let server = std::thread::spawn(move || {
        for step in 0..3 {
            let (stream, headers, body) = request(&server);
            assert!(headers.starts_with("post /v1beta/models/test:streamgeneratecontent"));
            let parts: Vec<_> = body["contents"]
                .as_array()
                .unwrap()
                .iter()
                .flat_map(|v| v["parts"].as_array().unwrap())
                .collect();
            if step > 0 {
                let calls: Vec<_> = parts
                    .iter()
                    .filter(|v| v.get("functionCall").is_some())
                    .collect();
                assert_eq!(calls[0]["thoughtSignature"], "signature-one");
                assert_eq!(calls.len(), if step == 1 { 2 } else { 3 });
                if step == 2 {
                    assert_eq!(calls[2]["thoughtSignature"], "signature-two");
                }
                assert_eq!(
                    parts
                        .iter()
                        .filter(|v| v.get("functionResponse").is_some())
                        .count(),
                    calls.len()
                );
            }
            let content = match step {
                0 => {
                    json!([{"functionCall":{"id":"a","name":"ping","args":{}},"thoughtSignature":"signature-one"},{"functionCall":{"id":"b","name":"ping","args":{}}}])
                }
                1 => {
                    json!([{"functionCall":{"id":"c","name":"ping","args":{}},"thoughtSignature":"signature-two"}])
                }
                _ => json!([{"text":"Complete"}]),
            };
            respond(
                stream,
                &[
                    json!({"candidates":[{"content":{"role":"model","parts":content},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":1,"candidatesTokenCount":1,"totalTokenCount":2}}),
                ],
                false,
            );
        }
    });
    let model = provider::builder(&profile, "local")
        .unwrap()
        .build()
        .model_handle()
        .clone();
    let host = TestHost::default();
    let result = run(
        &model,
        &profile,
        &host,
        Session::new(vec![Message::user("read")]),
        true,
        "",
    )
    .await;
    server.join().unwrap();
    assert_eq!(result.unwrap(), "Complete");
    assert_eq!(host.trace.lock().unwrap().len(), 6);
}
#[tokio::test]
async fn openai_stream_pairs_tool_call_ids_and_commits_visible_prefix() {
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    server.set_nonblocking(true).unwrap();
    let profile = Profile {
        endpoint: format!("http://{}/v1", server.local_addr().unwrap()),
        ..profile()
    };
    let server = std::thread::spawn(move || {
        for step in 0..2 {
            let (stream, _, body) = request(&server);
            assert_eq!(body["stream"], true);
            if step == 1 {
                let messages = body["messages"].as_array().unwrap();
                assert!(
                    messages
                        .iter()
                        .any(|m| m["role"] == "tool" && m["tool_call_id"] == "call-a")
                );
            }
            let delta = if step == 0 {
                json!({"role":"assistant","content":"先检查。","tool_calls":[{"index":0,"id":"call-a","type":"function","function":{"name":"ping","arguments":"{}"}}]})
            } else {
                json!({"role":"assistant","content":"完成。"})
            };
            respond(
                stream,
                &[
                    json!({"id":"reply","object":"chat.completion.chunk","created":0,"model":"test","choices":[{"index":0,"delta":delta,"finish_reason":null}]}),
                    json!({"id":"reply","object":"chat.completion.chunk","created":0,"model":"test","choices":[{"index":0,"delta":{},"finish_reason":if step==0 {"tool_calls"}else{"stop"}}]}),
                ],
                true,
            );
        }
    });
    let model = provider::builder(&profile, "local")
        .unwrap()
        .build()
        .model_handle()
        .clone();
    let host = TestHost::default();
    let result = run(
        &model,
        &profile,
        &host,
        Session::new(vec![Message::user("read")]),
        true,
        "",
    )
    .await;
    server.join().unwrap();
    assert_eq!(result.unwrap(), "完成。");
    assert!(
        host.events
            .lock()
            .unwrap()
            .iter()
            .any(|(kind, value)| kind == "assistant/partial" && value["delta"] == "先检查。")
    );
    let events = host.events.lock().unwrap();
    assert!(
        events
            .iter()
            .position(|(k, _)| k == "assistant/partial")
            .unwrap()
            < events.iter().position(|(k, _)| k == "tool/call").unwrap()
    );
}
