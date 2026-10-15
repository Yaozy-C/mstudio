use super::*;
use bytes::Bytes;
use rig_core::http_client::{HttpClientExt, Request};
use serde_json::json;
use std::io::{Read, Write};

pub(super) struct Captured {
    pub head: String,
    pub body: Vec<u8>,
}
pub(super) type Step = (u16, Box<dyn Fn(&Captured) -> Value + Send>);
pub(super) fn server(steps: Vec<Step>) -> (String, std::thread::JoinHandle<Vec<Captured>>) {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    listener.set_nonblocking(true).unwrap();
    let root = format!("http://{}", listener.local_addr().unwrap());
    let handle = std::thread::spawn(move || {
        let mut captured = Vec::new();
        for (status, response) in steps {
            let deadline = std::time::Instant::now() + Duration::from_secs(10);
            let mut stream = loop {
                if let Ok((stream, _)) = listener.accept() {
                    break stream;
                }
                assert!(
                    std::time::Instant::now() < deadline,
                    "missing expected test request"
                );
                std::thread::sleep(Duration::from_millis(5));
            };
            stream.set_nonblocking(false).unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut head = Vec::new();
            while !head.ends_with(b"\r\n\r\n") {
                let mut byte = [0];
                stream.read_exact(&mut byte).unwrap();
                head.push(byte[0]);
            }
            let head = String::from_utf8(head).unwrap();
            let size: usize = head
                .lines()
                .find_map(|line| {
                    line.to_ascii_lowercase()
                        .strip_prefix("content-length:")
                        .map(|s| s.trim().parse().unwrap())
                })
                .unwrap_or(0);
            let mut body = vec![0; size];
            stream.read_exact(&mut body).unwrap();
            let request = Captured { head, body };
            let payload = response(&request).to_string();
            write!(stream,"HTTP/1.1 {status} Test\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{payload}",payload.len()).unwrap();
            captured.push(request);
        }
        captured
    });
    (root, handle)
}
pub(super) fn upload(id: &'static str) -> Step {
    (
        200,
        Box::new(move |request| {
            assert!(request.head.starts_with("POST /files "));
            assert!(
                request
                    .head
                    .to_ascii_lowercase()
                    .contains("authorization: bearer test-key")
            );
            let text = String::from_utf8_lossy(&request.body);
            assert!(text.contains("name=\"purpose\"\r\n\r\nuser_data"));
            assert!(text.contains("expires_after[seconds]\"\r\n\r\n604800"));
            assert!(text.contains("hello"));
            let filename = text
                .split("filename=\"")
                .nth(1)
                .unwrap()
                .split('"')
                .next()
                .unwrap();
            json!({"id":id,"object":"file","bytes":5,"created_at":now(),"filename":filename,"purpose":"user_data","expires_at":now()+LIFETIME})
        }),
    )
}
pub(super) fn transport(root: &str, store: Option<&Store>) -> FileTransport {
    FileTransport::new(
        rig_reqwest::ReqwestClient::from(
            rig_http::Client::builder()
                .redirect(rig_http::redirect::Policy::none())
                .build()
                .unwrap(),
        ),
        Uploader::new(
            Arc::new(backends::deepseek::DeepSeek),
            root,
            "test-key",
            store,
        )
        .unwrap(),
    )
}
fn request(root: &str) -> Request<Bytes> {
    Request::builder()
        .method("POST")
        .uri(format!("{root}/responses"))
        .header("content-type", "application/json")
        .body(Bytes::from(super::tests::body().to_string()))
        .unwrap()
}

#[tokio::test]
async fn parallel_agents_share_one_upload_and_keep_canonical_pixels() {
    let (root, server) = server(vec![upload("file-api-once")]);
    let directory =
        std::env::temp_dir().join(format!("mstudio-files-parallel-{}", mstudio::media::id()));
    let store = Store::open(directory.clone()).unwrap();
    let first = Uploader::new(
        Arc::new(backends::deepseek::DeepSeek),
        &root,
        "test-key",
        Some(&store),
    )
    .unwrap();
    let second = Uploader::new(
        Arc::new(backends::deepseek::DeepSeek),
        &root,
        "test-key",
        Some(&store),
    )
    .unwrap();
    let original = super::tests::body().to_string().into_bytes();
    let before = original.clone();
    let (a, b) = tokio::join!(first.prepare(&original), second.prepare(&original));
    let a: Value = serde_json::from_slice(&a.unwrap().0).unwrap();
    let b: Value = serde_json::from_slice(&b.unwrap().0).unwrap();
    assert_eq!(a, b);
    assert_eq!(a["input"][0]["content"][0]["file_id"], "file-api-once");
    assert_eq!(original, before);
    assert_eq!(server.join().unwrap().len(), 1);
    drop(first);
    drop(second);
    drop(store);
    std::fs::remove_dir_all(directory).unwrap();
}

#[tokio::test]
async fn both_transports_recover_a_stale_id_once_and_reuse_the_new_id() {
    for streaming in [false, true] {
        let rejected: Step = (
            400,
            Box::new(|request| {
                let value: Value = serde_json::from_slice(&request.body).unwrap();
                assert_eq!(value["input"][0]["content"][0]["file_id"], "file-api-old");
                json!({"error":{"message":"file_id file-api-old expired"}})
            }),
        );
        let accepted = || -> Step {
            (
                200,
                Box::new(|request| {
                    let value: Value = serde_json::from_slice(&request.body).unwrap();
                    assert_eq!(
                        value["input"][0]["content"][0],
                        json!({"type":"input_image","file_id":"file-api-new"})
                    );
                    assert!(!String::from_utf8_lossy(&request.body).contains("base64"));
                    json!({"ok":true})
                }),
            )
        };
        let (root, server) = server(vec![
            upload("file-api-old"),
            rejected,
            upload("file-api-new"),
            accepted(),
            accepted(),
        ]);
        let client = transport(&root, None);
        if streaming {
            use futures::StreamExt;
            for _ in 0..2 {
                let mut response = client.send_streaming(request(&root)).await.unwrap();
                while let Some(chunk) = response.body_mut().next().await {
                    chunk.unwrap();
                }
            }
        } else {
            for _ in 0..2 {
                let response = client.send::<_, Bytes>(request(&root)).await.unwrap();
                assert_eq!(response.status(), 200);
            }
        }
        assert_eq!(server.join().unwrap().len(), 5);
    }
}

#[tokio::test]
async fn upload_auth_failure_has_no_inline_fallback() {
    let (root, server) = server(vec![(
        401,
        Box::new(|_| json!({"error":{"message":"unauthorized"}})),
    )]);
    let failure = transport(&root, None)
        .send::<_, Bytes>(request(&root))
        .await
        .err()
        .unwrap();
    let Error::InvalidStatusCodeWithDetails { status, .. } = failure else {
        panic!("lost HTTP status")
    };
    assert_eq!(status, 401);
    assert_eq!(server.join().unwrap().len(), 1);
}

#[tokio::test]
async fn stale_id_recovery_is_bounded_and_second_bad_mapping_is_removed() {
    let rejected = || -> Step {
        (
            400,
            Box::new(|_| json!({"error":{"message":"file_id expired"}})),
        )
    };
    let (root, server) = server(vec![
        upload("file-api-old"),
        rejected(),
        upload("file-api-new"),
        rejected(),
    ]);
    let client = transport(&root, None);
    assert!(client.send::<_, Bytes>(request(&root)).await.is_err());
    let image = projection::collect(&super::tests::body())
        .unwrap()
        .remove(0)
        .1;
    assert!(
        index::get(
            &client.files.db.lock().unwrap(),
            &client.files.scope,
            &image.variant,
            now()
        )
        .unwrap()
        .is_none()
    );
    assert_eq!(server.join().unwrap().len(), 4);
}
