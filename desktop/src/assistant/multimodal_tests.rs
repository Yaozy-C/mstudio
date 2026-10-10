use super::{
    agent,
    attachment_tests::{fixture, multimodal},
    attachments::{self, Reference},
    context, media_input,
};
use serde_json::{Value, json};
use std::io::{Read, Write};

fn mock(response: Value) -> (String, std::thread::JoinHandle<String>) {
    let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    server.set_nonblocking(true).unwrap();
    let address = format!("http://{}", server.local_addr().unwrap());
    let task = std::thread::spawn(move || {
        let start = std::time::Instant::now();
        let mut stream = loop {
            if let Ok((s, _)) = server.accept() {
                break s;
            }
            assert!(start.elapsed().as_secs() < 10, "request not received");
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        stream.set_nonblocking(false).unwrap();
        stream
            .set_read_timeout(Some(std::time::Duration::from_secs(10)))
            .unwrap();
        let mut bytes = vec![];
        let mut buf = [0; 4096];
        loop {
            let n = stream.read(&mut buf).unwrap();
            assert!(n > 0);
            bytes.extend_from_slice(&buf[..n]);
            if let Some(pos) = bytes.windows(4).position(|w| w == b"\r\n\r\n") {
                let header = String::from_utf8_lossy(&bytes[..pos]).to_lowercase();
                let length = header
                    .lines()
                    .find_map(|l| {
                        l.strip_prefix("content-length:")?
                            .trim()
                            .parse::<usize>()
                            .ok()
                    })
                    .unwrap_or(0);
                if bytes.len() >= pos + 4 + length {
                    break;
                }
            }
        }
        let body = response.to_string();
        write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        String::from_utf8(bytes).unwrap()
    });
    (address, task)
}

#[tokio::test]
async fn gemini_transmits_original_video_audio_and_image() {
    let (root, store, doc) = fixture();
    let (address, request) = mock(
        json!({"candidates":[{"content":{"role":"model","parts":[{"text":"understood"}]},"finishReason":"STOP"}],"usageMetadata":{"promptTokenCount":1,"candidatesTokenCount":1,"totalTokenCount":2}}),
    );
    let mut profile = multimodal();
    profile.endpoint = address;
    profile.model = "test".into();
    let refs = ["image", "audio", "video"].map(|id| Reference {
        kind: "asset".into(),
        id: id.into(),
    });
    let payload = attachments::payload(&store, &doc, "Read references", &refs, &profile).unwrap();
    let result = agent::complete(&profile, "test-key", super::messages(&[], payload), None).await;
    let request = request.join().unwrap();
    assert!(request.starts_with("POST /v1beta/models/test:generateContent"));
    for mime in ["image/jpeg", "audio/mp3", "video/mp4"] {
        assert!(request.contains(mime), "missing {mime}: {request}");
    }
    assert!(request.contains("dGVzdC1zb3VyY2UtY29udGVudA=="));
    assert_eq!(result.unwrap(), "understood");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn claude_transmits_pdf_and_parses_response() {
    let (root, store, _) = fixture();
    let input = root.join("brief.pdf");
    std::fs::write(&input, b"%PDF-1.4\nexample").unwrap();
    let asset = mstudio::text_asset::import(&input, &root).unwrap();
    let (address, request) = mock(
        json!({"id":"msg_test","type":"message","role":"assistant","model":"test","content":[{"type":"text","text":"read pdf"}],"stop_reason":"end_turn","stop_sequence":null,"usage":{"input_tokens":1,"output_tokens":1}}),
    );
    let mut profile = multimodal();
    profile.adapter = "anthropic-native".into();
    profile.endpoint = format!("{address}/v1");
    profile.model = "test".into();
    let payload = json!(media_input::parts(&store, &asset, &profile).unwrap());
    assert!(!context::text_only(&payload).to_string().contains("base64"));
    let result = agent::complete(&profile, "test-key", super::messages(&[], payload), None).await;
    let request = request.join().unwrap();
    assert!(request.starts_with("POST /v1/messages"));
    assert!(request.to_lowercase().contains("x-api-key: test-key"));
    assert!(request.contains("application/pdf"));
    assert_eq!(result.unwrap(), "read pdf");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn openai_audio_is_input_audio_not_metadata() {
    let (root, store, _) = fixture();
    let (address, request) = mock(
        json!({"id":"test","object":"chat.completion","created":0,"model":"test","choices":[{"index":0,"message":{"role":"assistant","content":"heard"},"finish_reason":"stop"}],"usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}}),
    );
    let mut profile = multimodal();
    profile.adapter = "openai-compatible".into();
    profile.endpoint = format!("{address}/v1");
    profile.model = "test".into();
    let asset = store
        .assets()
        .unwrap()
        .into_iter()
        .find(|a| a.kind == "audio")
        .unwrap();
    let payload = json!(media_input::parts(&store, &asset, &profile).unwrap());
    let result = agent::complete(&profile, "", super::messages(&[], payload), None).await;
    let request = request.join().unwrap();
    assert!(request.contains("input_audio"));
    assert!(request.contains("mp3"));
    assert!(request.contains("dGVzdC1zb3VyY2UtY29udGVudA=="));
    assert_eq!(result.unwrap(), "heard");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn text_files_work_without_vision_and_binary_inputs_fail_closed() {
    let (root, store, _) = fixture();
    let path = root.join("brief.md");
    std::fs::write(&path, "商品容量 10L；只引用这条已确认事实。").unwrap();
    let asset = mstudio::text_asset::import(&path, &root).unwrap();
    let mut profile = super::config::Profile::default();
    let parts = media_input::parts(&store, &asset, &profile).unwrap();
    assert!(parts[1]["text"].as_str().unwrap().contains("商品容量 10L"));
    for asset in store.assets().unwrap() {
        assert!(media_input::parts(&store, &asset, &profile).is_err());
    }
    profile = multimodal();
    profile.adapter = "anthropic-native".into();
    let audio = store
        .assets()
        .unwrap()
        .into_iter()
        .find(|a| a.kind == "audio")
        .unwrap();
    assert!(media_input::parts(&store, &audio, &profile).is_ok());
    profile.inputs.audio = false;
    assert!(media_input::parts(&store, &audio, &profile).is_err());
    let mut foreign = asset;
    foreign.path = path.to_string_lossy().into();
    assert!(media_input::parts(&store, &foreign, &profile).is_err());
    std::fs::write(&path, vec![b'a'; 120_001]).unwrap();
    assert!(mstudio::text_asset::import(&path, &root).is_err());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn responses_transmits_pdf_as_input_file() {
    let (root, store, _) = fixture();
    let input = root.join("brief.pdf");
    std::fs::write(&input, b"%PDF-1.4\nexample").unwrap();
    let asset = mstudio::text_asset::import(&input, &root).unwrap();
    let (address, request) = mock(
        json!({"id":"resp_test","object":"response","created_at":0,"status":"completed","model":"test","output":[{"id":"msg_test","type":"message","role":"assistant","status":"completed","content":[{"type":"output_text","text":"read file","annotations":[]}]}],"usage":{"input_tokens":1,"output_tokens":1,"total_tokens":2}}),
    );
    let mut profile = multimodal();
    profile.adapter = "openai-responses".into();
    profile.endpoint = format!("{address}/v1");
    profile.model = "test".into();
    let payload = json!(media_input::parts(&store, &asset, &profile).unwrap());
    let result = agent::complete(&profile, "test-key", super::messages(&[], payload), None).await;
    let request = request.join().unwrap();
    assert!(request.starts_with("POST /v1/responses"));
    assert!(request.contains("input_file"));
    assert!(request.contains("data:application/pdf;base64,"));
    assert_eq!(result.unwrap(), "read file");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[tokio::test]
async fn compatible_video_configuration_transmits_original_video() {
    let (root, store, doc) = fixture();
    let (address, request) = mock(json!({
        "id":"test", "object":"chat.completion", "created":0, "model":"test",
        "choices":[{"index":0,"message":{"role":"assistant","content":"watched"},"finish_reason":"stop"}],
        "usage":{"prompt_tokens":1,"completion_tokens":1,"total_tokens":2}
    }));
    let mut profile = multimodal();
    profile.adapter = "openai-compatible".into();
    profile.endpoint = format!("{address}/v1");
    profile.model = "custom-video-model".into();
    let refs = [Reference {
        kind: "asset".into(),
        id: "video".into(),
    }];
    let payload = attachments::payload(&store, &doc, "Read video", &refs, &profile).unwrap();
    assert!(!payload.to_string().contains("request frames"));
    let result = agent::complete(&profile, "test-key", super::messages(&[], payload), None).await;
    let request = request.join().unwrap();
    let body: Value = serde_json::from_str(request.split_once("\r\n\r\n").unwrap().1).unwrap();
    let part = body["messages"]
        .as_array()
        .unwrap()
        .iter()
        .filter_map(|m| m["content"].as_array())
        .flatten()
        .find(|p| p["type"] == "video_url")
        .unwrap();
    assert_eq!(
        part["video_url"]["url"],
        "data:video/mp4;base64,dGVzdC1zb3VyY2UtY29udGVudA=="
    );
    assert_eq!(result.unwrap(), "watched");
    profile.inputs.video = false;
    profile.inputs.image = false;
    assert!(attachments::payload(&store, &doc, "Read video", &refs, &profile).is_err());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
