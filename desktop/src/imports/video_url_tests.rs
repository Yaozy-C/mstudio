use super::*;
#[test]
fn accepts_page_links_and_share_text_without_treating_it_as_a_command() {
    assert_eq!(
        parse_url("打开 https://v.douyin.com/abc/ 复制打开抖音")
            .unwrap()
            .as_str(),
        "https://v.douyin.com/abc/"
    );
    assert_eq!(
        parse_url("https://youtu.be/abc?v=1&list=2")
            .unwrap()
            .query(),
        Some("v=1&list=2")
    );
    for url in [
        "file:///tmp/a.mp4",
        "https://user:password@example.com/a",
        "not a link",
    ] {
        assert!(parse_url(url).is_err());
    }
}
#[tokio::test]
async fn direct_download_keeps_exact_bytes_and_rejects_oversized_files() {
    use std::io::{Read, Write};
    use std::net::TcpListener;
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let url = reqwest::Url::parse(&format!(
        "http://{}/original.mp4",
        listener.local_addr().unwrap()
    ))
    .unwrap();
    let server = std::thread::spawn(move || {
        for oversized in [false, true] {
            let (mut socket, _) = listener.accept().unwrap();
            let mut buf = [0; 2048];
            let received = socket.read(&mut buf).unwrap();
            assert!(received > 0, "expected an HTTP request before responding");
            let length = if oversized { 1024_u64.pow(3) + 1 } else { 4 };
            write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n"
            )
            .unwrap();
            if !oversized {
                socket.write_all(b"raw!").unwrap();
            }
        }
    });
    let work = std::env::temp_dir().join(media::id());
    std::fs::create_dir_all(&work).unwrap();
    let (file, _) = download(&url, &work, &work.join("cache")).await.unwrap();
    assert_eq!(std::fs::read(file).unwrap(), b"raw!");
    assert!(download(&url, &work, &work.join("cache")).await.is_err());
    assert!(!work.join("cache").exists());
    server.join().unwrap();
    std::fs::remove_dir_all(work).unwrap();
}
#[tokio::test]
#[ignore = "downloads the official yt-dlp executable; serves a synthetic local video page"]
async fn official_extractor_downloads_a_page_video_with_sound() {
    use std::io::{Read, Write};
    use std::sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    };
    let root = std::env::temp_dir().join(format!("mstudio-url-page-{}", media::id()));
    std::fs::create_dir_all(&root).unwrap();
    let source = root.join("original.mp4");
    media::run(
        media::command("ffmpeg")
            .args([
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=32x32:r=10",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440",
                "-t",
                "0.3",
                "-c:v",
                "libx264",
                "-c:a",
                "aac",
            ])
            .arg(&source),
    )
    .unwrap();
    let video = std::fs::read(&source).unwrap();
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let url =
        reqwest::Url::parse(&format!("http://{}/watch", listener.local_addr().unwrap())).unwrap();
    listener.set_nonblocking(true).unwrap();
    let stop = Arc::new(AtomicBool::new(false));
    let stopped = stop.clone();
    let server = std::thread::spawn(move || {
        while !stopped.load(Ordering::Relaxed) {
            let Ok((mut socket, _)) = listener.accept() else {
                std::thread::sleep(Duration::from_millis(20));
                continue;
            };
            socket
                .set_read_timeout(Some(Duration::from_secs(2)))
                .unwrap();
            let mut request = [0; 4096];
            let size = socket.read(&mut request).unwrap_or(0);
            let request = String::from_utf8_lossy(&request[..size]);
            let (mime, body): (&str, &[u8]) = if request.contains("/original.mp4") {
                ("video/mp4", &video)
            } else {
                ("text/html", b"<html><title>Original test video</title><video controls src='/original.mp4'></video></html>")
            };
            let _ = write!(
                socket,
                "HTTP/1.1 200 OK\r\nContent-Type: {mime}\r\nContent-Length: {}\r\nConnection: close\r\n\r\n",
                body.len()
            );
            if !request.starts_with("HEAD ") {
                let _ = socket.write_all(body);
            }
        }
    });
    let work = root.join("work");
    std::fs::create_dir_all(&work).unwrap();
    let result = download(
        &url,
        &work,
        &std::env::temp_dir().join("mstudio-ytdlp-test-cache"),
    )
    .await;
    stop.store(true, Ordering::Relaxed);
    server.join().unwrap();
    let (path, _) = result.unwrap();
    assert_eq!(
        std::fs::read(path.clone()).unwrap(),
        std::fs::read(source).unwrap()
    );
    let probe = media::probe(&path).unwrap();
    for kind in ["video", "audio"] {
        assert!(
            probe["streams"]
                .as_array()
                .unwrap()
                .iter()
                .any(|s| s["codec_type"] == kind)
        );
    }
    std::fs::remove_dir_all(root).unwrap();
}
