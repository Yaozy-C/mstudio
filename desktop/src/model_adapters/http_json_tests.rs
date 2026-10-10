use super::*;
#[test]
fn maps_single_and_multiple_outputs_and_rejects_invalid_results() {
    let config = json!({"outputPointer":"/data","itemUrlPointer":"/url","kind":"video"});
    let result = complete(
        &json!({"data":[{"url":"https://example.com/movie.mp4"}]}),
        &config,
    )
    .unwrap();
    assert_eq!(result.status, "COMPLETED");
    assert_eq!(result.outputs.unwrap()[0].kind, "video");
    assert!(complete(&json!({"data":[]}), &config).is_err());
    assert!(complete(&json!({"data":[{"url":"file:///etc/passwd"}]}), &config).is_err());
    assert!(validate(&json!({"outputPointer":"data"})).is_err());
    assert!(validate(&json!({"outputPointer":"/url","pollUrl":"/tasks/{id}"})).is_err());
}
#[test]
fn declared_headers_are_validated_and_never_override_the_connection() {
    let accepted =
        json!({"outputPointer":"/url","headers":{"X-DashScope-Async":"enable","api-key":"local"}});
    assert_eq!(header_pairs(&accepted).unwrap().len(), 2);
    assert!(validate(&accepted).is_ok());
    for rejected in [
        json!({"outputPointer":"/url","headers":{"Authorization":"Bearer leaked"}}),
        json!({"outputPointer":"/url","headers":{"host":"evil.example"}}),
        json!({"outputPointer":"/url","headers":{"X-Injected":"line\r\nX-Other: 1"}}),
        json!({"outputPointer":"/url","headers":{"X Bad Name":"value"}}),
        json!({"outputPointer":"/url","headers":{"X-Empty":""}}),
        json!({"outputPointer":"/url","headers":{"X-Number":1}}),
        json!({"outputPointer":"/url","headers":[]}),
        json!({"outputPointer":"/url","headers":{
            "X-1":"v","X-2":"v","X-3":"v","X-4":"v","X-5":"v",
            "X-6":"v","X-7":"v","X-8":"v","X-9":"v"}}),
    ] {
        assert!(validate(&rejected).is_err(), "{rejected} 应被拒绝");
    }
}
#[test]
fn polling_encodes_ids_and_never_forwards_credentials_to_other_origins() {
    assert_eq!(
        poll_url("https://example.com/generate", "/tasks/{id}", "a/b +?").unwrap(),
        "https://example.com/tasks/a%2Fb%20%2B%3F"
    );
    assert!(
        poll_url(
            "https://example.com/generate",
            "https://other.com/{id}",
            "x"
        )
        .is_err()
    );
    assert!(endpoint("http://example.com/api").is_err());
    assert!(endpoint("http://127.0.0.1:8188/api").is_ok());
    assert!(endpoint("https://user:secret@example.com/api").is_err());
}
#[tokio::test]
async fn custom_provider_submits_and_polls_a_real_http_queue() {
    use std::io::{Read, Write};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let base = format!("http://{}", listener.local_addr().unwrap());
    let server = std::thread::spawn(move || {
        for (method, body) in [
            ("POST /generate", r#"{"id":"job-1"}"#),
            (
                "GET /tasks/job-1",
                r#"{"state":"done","url":"https://example.com/result.png"}"#,
            ),
        ] {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(std::time::Duration::from_secs(5)))
                .unwrap();
            let mut buffer = [0u8; 8192];
            let n = stream.read(&mut buffer).unwrap();
            let request = String::from_utf8_lossy(&buffer[..n]);
            let lower = request.to_lowercase();
            assert!(request.starts_with(method));
            assert!(lower.contains("authorization: bearer test-secret"));
            assert!(lower.contains("x-dashscope-async: enable"));
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        }
    });
    let config = json!({"outputPointer":"/url","idPointer":"/id","pollUrl":"/tasks/{id}","statusPointer":"/state","doneValue":"done","headers":{"X-DashScope-Async":"enable"}});
    let endpoint = format!("{base}/generate");
    let submitted = HttpJson
        .submit(
            "test-secret",
            &endpoint,
            &json!({"prompt":"test"}),
            &config,
            None,
        )
        .await
        .unwrap();
    let mut job = json!({"endpoint":endpoint,"providerConfig":config});
    submitted.apply(&mut job).unwrap();
    assert_eq!(job["status"], "IN_QUEUE");
    let done = HttpJson.refresh("test-secret", &job).await.unwrap();
    assert_eq!(done.status, "COMPLETED");
    assert_eq!(
        done.outputs.unwrap()[0].url,
        "https://example.com/result.png"
    );
    server.join().unwrap();
}
