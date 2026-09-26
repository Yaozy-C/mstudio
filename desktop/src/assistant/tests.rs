use super::{
    agent,
    config::{self, Profile},
    history, messages,
    pending::PendingTurn,
};
use crate::database::Store;
use serde_json::json;
#[test]
fn config_and_project_history_are_isolated() {
    let mut p = Profile {
        endpoint: "http://127.0.0.1:1234/v1".into(),
        model: "local-model".into(),
        adapter: "openai-compatible".into(),
        context_window: None,
        inputs: Default::default(),
    };
    assert!(config::validate(&p).is_ok());
    p.endpoint = "http://example.com/v1".into();
    assert!(config::validate(&p).is_err());
    p.endpoint = "https://example.com/v1?key=secret".into();
    assert!(config::validate(&p).is_err());
    let dir = std::env::temp_dir().join(format!("mstudio-agent-test-{}", std::process::id()));
    let store = Store::open(dir.clone()).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','test','{}',0)", [])
        .unwrap();
    history::append(&store, "p", "test", &json!("test"), "reply", "local-model").unwrap();
    assert_eq!(history::read(&store, "p").unwrap().len(), 2);
    assert!(history::read(&store, "other").unwrap().is_empty());
    let input = messages(&history::read(&store, "p").unwrap(), json!("next"));
    assert_eq!(input.as_array().unwrap().len(), 5);
    let turn = PendingTurn::begin("test-turn").unwrap();
    assert!(PendingTurn::begin("test-turn").is_err());
    super::pending::cancel_assistant("test-turn".into());
    assert!(turn.token.is_cancelled());
    drop(turn);
    assert!(PendingTurn::begin("test-turn").is_ok());
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}
#[tokio::test]
async fn rig_roundtrip_uses_local_http_and_real_provider_parser() {
    use std::io::{Read, Write};
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let address = server.local_addr().unwrap();
    let task = std::thread::spawn(move || {
        let (mut stream, _) = server.accept().unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let mut bytes = Vec::new();
        let mut buf = [0; 4096];
        loop {
            let n = stream.read(&mut buf).unwrap();
            bytes.extend_from_slice(&buf[..n]);
            if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let headers = String::from_utf8_lossy(&bytes[..pos]).to_lowercase();
                let len = headers
                    .lines()
                    .find_map(|l| {
                        l.strip_prefix("content-length:")
                            .and_then(|s| s.trim().parse::<usize>().ok())
                    })
                    .unwrap_or(0);
                if bytes.len() >= pos + 4 + len {
                    break;
                }
            }
        }
        let request = String::from_utf8_lossy(&bytes);
        assert!(request.starts_with("POST /v1/chat/completions"));
        assert!(request.contains("local-test"));
        assert!(request.contains("storyboard"));
        assert!(request.contains("Keep the main character."));
        assert_eq!(request.matches("data:image/jpeg;base64,").count(), 1);
        let body=json!({"id":"test","object":"chat.completion","created":0,"model":"local-test","choices":[{"index":0,"message":{"role":"assistant","content":"Local test reply"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}).to_string();
        write!(stream,"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}",body.len(),body).unwrap();
    });
    let profile = Profile {
        endpoint: format!("http://{address}/v1"),
        model: "local-test".into(),
        adapter: "openai-compatible".into(),
        context_window: None,
        inputs: crate::assistant::config::Inputs {
            image: true,
            ..Default::default()
        },
    };
    let (root, store, doc) = super::attachment_tests::fixture();
    let refs =
        [("node", "script"), ("asset", "image")].map(|(kind, id)| super::attachments::Reference {
            kind: kind.into(),
            id: id.into(),
        });
    let payload = super::attachments::payload(&store, &doc, "storyboard", &refs, &profile).unwrap();
    let result = agent::complete(&profile, "", messages(&[], payload), None).await;
    task.join().unwrap();
    assert_eq!(result.unwrap(), "Local test reply");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
