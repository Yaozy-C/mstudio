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
    attachment_only: Option<bool>,
) -> Result<Asset, String> {
    let token = CancellationToken::new();
    {
        let mut active = active().lock().unwrap();
        if active.contains_key(&id) {
            return Err("下载正在进行".into());
        }
        active.insert(id.clone(), token.clone());
    }
    let result = import(
        &app,
        &project_id,
        &url,
        &token,
        attachment_only.unwrap_or(false),
    )
    .await;
    active().lock().unwrap().remove(&id);
    result.map_err(|e| crate::app_error::wire(e, "ASSET_IMPORT_FAILED", "import"))
}
async fn import(
    app: &tauri::AppHandle,
    project: &str,
    raw: &str,
    token: &CancellationToken,
    attachment_only: bool,
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
            super::import_source_scoped(&owner.state::<Store>(), &project, &path, &name, &staging, attachment_only)
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
pub(crate) fn parse_url(raw: &str) -> Result<reqwest::Url> {
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
pub(crate) async fn download(
    url: &reqwest::Url,
    work: &Path,
    cache: &Path,
) -> Result<(PathBuf, String)> {
    download_with_progress(url, work, cache, &|_, _, _| {}).await
}
pub(crate) async fn download_with_progress(
    url: &reqwest::Url,
    work: &Path,
    cache: &Path,
    progress: &(dyn Fn(u64, Option<u64>, bool) + Send + Sync),
) -> Result<(PathBuf, String)> {
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
        stream_file(response, &path, progress).await?;
        return Ok((path, format!("链接视频.{extension}")));
    }
    let executable = video_downloader::executable(cache)
        .await
        .context("无法准备视频下载器，请检查网络后重试")?;
    let output = super::video_download_progress::run(
        video_downloader::progress_command(&executable, work, url.as_str()),
        progress,
    )
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
async fn stream_file(
    mut response: reqwest::Response,
    path: &Path,
    progress: &(dyn Fn(u64, Option<u64>, bool) + Send + Sync),
) -> Result<()> {
    let total = response.content_length().filter(|n| *n > 0);
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
        progress(size, total, false);
    }
    ensure!(size > 0, "下载的视频为空");
    Ok(())
}
#[cfg(test)]
#[path = "video_url_tests.rs"]
mod tests;
