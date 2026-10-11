use super::video_downloader;
use crate::{database::Store, project_storage};
use anyhow::{Context, Result, ensure};
use mstudio::{media, model::Asset};
use std::{
    collections::HashMap,
    io::Write,
    path::{Path, PathBuf},
    sync::{Mutex, OnceLock},
    time::Duration,
};
use tauri::Manager;
use tokio_util::sync::CancellationToken;

fn active() -> &'static Mutex<HashMap<String, CancellationToken>> {
    static ACTIVE: OnceLock<Mutex<HashMap<String, CancellationToken>>> = OnceLock::new();
    ACTIVE.get_or_init(Default::default)
}
#[tauri::command]
pub fn cancel_video_import(id: String) {
    if let Some(token) = active().lock().unwrap().get(&id) {
        token.cancel();
    }
}
#[tauri::command]
pub async fn import_video_url(
    app: tauri::AppHandle,
    project_id: String,
    id: String,
    url: String,
) -> Result<Asset, String> {
    let token = CancellationToken::new();
    {
        let mut active = active().lock().unwrap();
        if active.contains_key(&id) {
            return Err("下载正在进行".into());
        }
        active.insert(id.clone(), token.clone());
    }
    let result = import(&app, &project_id, &url, &token).await;
    active().lock().unwrap().remove(&id);
    result.map_err(|e| crate::app_error::wire(e, "ASSET_IMPORT_FAILED", "import"))
}
async fn import(
    app: &tauri::AppHandle,
    project: &str,
    raw: &str,
    token: &CancellationToken,
) -> Result<Asset> {
    let url = parse_url(raw)?;
    let store = app.state::<Store>();
    let _guard = project_storage::working(&store, project).await?;
    let work = store.media_root().join("imports").join(media::id());
    project_storage::track_file(&store, project, &work)?;
    std::fs::create_dir_all(&work)?;
    let result = async {
        let cache = app.path().app_cache_dir()?.join("video-downloader");
        let downloaded = tokio::select! {
            _ = token.cancelled() => anyhow::bail!("已取消视频下载"),
            result = tokio::time::timeout(Duration::from_secs(20 * 60), download(&url, &work, &cache)) => result.context("视频下载超时，请重试")??,
        };
        let project = project.to_owned();
        let owner = app.clone();
        let staging = work.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let (path, name) = downloaded;
            // Probe before committing: HTML, audio-only files and corrupt downloads never enter the project.
            ensure!(media::probe(&path)?["streams"].as_array().is_some_and(|s| s.iter().any(|s| s["codec_type"] == "video")), "链接未返回有效视频");
            super::import_source(&owner.state::<Store>(), &project, &path, &name, &staging)
        }).await?
    }.await;
    let cleanup = super::discard(&store, project, &work);
    match result {
        Ok(asset) => {
            cleanup?;
            crate::project_service::notify(app, project);
            Ok(asset)
        }
        Err(error) => {
            let _ = cleanup;
            Err(error)
        }
    }
}
fn parse_url(raw: &str) -> Result<reqwest::Url> {
    ensure!(raw.len() <= 16_000, "视频链接过长");
    let start = raw
        .find("https://")
        .or_else(|| raw.find("http://"))
        .context("请输入 HTTP 或 HTTPS 视频链接")?;
    let value = raw[start..]
        .split_whitespace()
        .next()
        .unwrap_or_default()
        .trim_end_matches(['，', '。', '；', ')', '）', '"', '\'', '>']);
    let url = reqwest::Url::parse(value).context("视频链接无效")?;
    ensure!(
        ["http", "https"].contains(&url.scheme()) && url.host_str().is_some(),
        "请输入 HTTP 或 HTTPS 视频链接"
    );
    ensure!(
        url.username().is_empty() && url.password().is_none(),
        "视频链接不能包含登录凭据"
    );
    Ok(url)
}
async fn download(url: &reqwest::Url, work: &Path, cache: &Path) -> Result<(PathBuf, String)> {
    // Direct files avoid installing the page extractor.
    let extension = Path::new(url.path())
        .extension()
        .and_then(|v| v.to_str())
        .unwrap_or("")
        .to_lowercase();
    if ["mp4", "mov", "m4v", "webm", "mkv"].contains(&extension.as_str()) {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(20 * 60))
            .build()?;
        let response = client.get(url.clone()).send().await?.error_for_status()?;
        let path = work.join(format!("source.{extension}"));
        stream_file(response, &path).await?;
        return Ok((path, format!("链接视频.{extension}")));
    }
    let executable = video_downloader::executable(cache)
        .await
        .context("无法准备视频下载器，请检查网络后重试")?;
    let output = video_downloader::command(&executable, work, url.as_str())
        .output()
        .await
        .context("无法启动视频下载器")?;
    ensure!(
        output.status.success(),
        "视频下载失败：{}",
        String::from_utf8_lossy(&output.stderr)
            .chars()
            .rev()
            .take(1800)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>()
    );
    let path = std::fs::read_dir(work)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .find(|p| {
            p.is_file()
                && ["mp4", "mov", "m4v", "webm", "mkv"]
                    .contains(&p.extension().and_then(|s| s.to_str()).unwrap_or(""))
        })
        .context("未下载到视频；链接可能需要登录、已失效或不是单条视频")?;
    super::validate(
        path.file_name()
            .unwrap()
            .to_str()
            .context("视频文件名无效")?,
        std::fs::metadata(&path)?.len(),
    )?;
    let title = std::fs::read_to_string(work.join("source.info.json"))
        .ok()
        .and_then(|s| serde_json::from_str::<serde_json::Value>(&s).ok())
        .and_then(|v| v["title"].as_str().map(str::to_owned))
        .unwrap_or_else(|| "链接视频".into());
    let name = format!(
        "{}.{}",
        title
            .chars()
            .filter(|c| !c.is_control() && !['/', '\\'].contains(c))
            .take(160)
            .collect::<String>(),
        path.extension().unwrap().to_string_lossy()
    );
    Ok((path, name))
}
async fn stream_file(mut response: reqwest::Response, path: &Path) -> Result<()> {
    const MAX: u64 = 1024 * 1024 * 1024;
    ensure!(
        response.content_length().unwrap_or(0) <= MAX,
        "视频超过 1 GiB"
    );
    let mut file = std::fs::File::create(path)?;
    let mut size = 0;
    while let Some(chunk) = response.chunk().await? {
        size += chunk.len() as u64;
        ensure!(size <= MAX, "视频超过 1 GiB");
        file.write_all(&chunk)?;
    }
    ensure!(size > 0, "下载的视频为空");
    Ok(())
}
#[cfg(test)]
mod tests {
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
                socket.read(&mut buf).unwrap();
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
        let url = reqwest::Url::parse(&format!("http://{}/watch", listener.local_addr().unwrap()))
            .unwrap();
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
}
