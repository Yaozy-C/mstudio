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
            assert!(request.starts_with(method));
            assert!(
                request
                    .to_lowercase()
                    .contains("authorization: bearer test-secret")
            );
            write!(stream, "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{}", body.len(), body).unwrap();
        }
    });
    let config = json!({"outputPointer":"/url","idPointer":"/id","pollUrl":"/tasks/{id}","statusPointer":"/state","doneValue":"done"});
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
