use super::{Session, Source, Video, progress::Reporter};
use anyhow::{Context, Result, ensure};
use std::path::{Path, PathBuf};
use tauri::Manager;
use tokio_util::sync::CancellationToken;

pub(super) async fn prepare(
    app: &tauri::AppHandle,
    directory: &Path,
    source: Source,
    progress: &Reporter,
) -> Result<Option<Video>> {
    let (path, name) = match source {
        Source::File => {
            progress.stage("selecting");
            let Some(file) = rfd::AsyncFileDialog::new()
                .add_filter("Video", &["mp4", "mov", "m4v", "webm", "mkv"])
                .pick_file()
                .await
            else {
                return Ok(None);
            };
            let source = file.path().to_owned();
            let name = file.file_name();
            crate::imports::validate(&name, std::fs::metadata(&source)?.len())?;
            let extension = source.extension().context("视频文件没有扩展名")?;
            let path = directory.join(format!("original.{}", extension.to_string_lossy()));
            let target = path.clone();
            progress.stage("copying");
            let reporter = progress.clone();
            tauri::async_runtime::spawn_blocking(move || copy_file(&source, &target, &reporter))
                .await??;
            (path, name)
        }
        Source::Url { url } => {
            let url = crate::imports::video_url::parse_url(&url)?;
            let cache = app.path().app_cache_dir()?.join("video-downloader");
            progress.stage("connecting");
            crate::imports::video_url::download_with_progress(
                &url,
                directory,
                &cache,
                &|done, total, fragments| progress.download(done, total, fragments),
            )
            .await?
        }
    };
    progress.stage("checking");
    let probe_path = path.clone();
    let metadata =
        tauri::async_runtime::spawn_blocking(move || mstudio::media::probe(&probe_path)).await??;
    let streams = metadata["streams"].as_array().context("无法读取视频轨道")?;
    let stream = streams
        .iter()
        .find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1)
        .context("文件不包含视频轨道")?;
    let duration = metadata["format"]["duration"]
        .as_str()
        .and_then(|s| s.parse::<f64>().ok())
        .context("无法读取视频时长")?;
    ensure!(
        duration.is_finite() && duration > 0. && duration <= 600.,
        "请使用 10 分钟以内的视频"
    );
    let (width, height) = mstudio::media::picture_dimensions(stream)?;
    Ok(Some(Video {
        id: String::new(),
        name,
        path: path.to_string_lossy().into(),
        duration,
        width: width.into(),
        height: height.into(),
        has_audio: streams.iter().any(|s| s["codec_type"] == "audio"),
    }))
}

/// Retain the original for playback; only the full-duration model input is compressed.
pub(super) async fn model_copy(
    session: &Session,
    token: &CancellationToken,
    progress: &Reporter,
) -> Result<PathBuf> {
    const LIMIT: u64 = 12 * 1024 * 1024;
    let original = Path::new(&session.video.path);
    let extension = original
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    if std::fs::metadata(original)?.len() <= LIMIT
        && ["mp4", "m4v", "mov", "webm"].contains(&extension.as_str())
    {
        return Ok(original.into());
    }
    let path = session.directory.join("analysis.mp4");
    if path.is_file() && std::fs::metadata(&path)?.len() <= LIMIT {
        return Ok(path);
    }
    let bitrate = ((10. * 1024. * 1024. * 8. / session.video.duration) as u64)
        .saturating_sub(96_000)
        .clamp(40_000, 4_000_000);
    let pending = session.directory.join("analysis-pending.mp4");
    let mut command = tokio::process::Command::new(mstudio::media::binary("ffmpeg"));
    command
        .kill_on_drop(true)
        .args(["-v", "error", "-nostdin", "-y", "-progress", "pipe:1", "-i"])
        .arg(original)
        .args([
            "-map",
            "0:v:0",
            "-map",
            "0:a:0?",
            "-vf",
            "scale=960:960:force_original_aspect_ratio=decrease:force_divisible_by=2",
            "-r",
            "24",
            "-c:v",
            "libx264",
            "-preset",
            "fast",
            "-b:v",
        ])
        .arg(bitrate.to_string())
        .args(["-maxrate"])
        .arg(bitrate.to_string())
        .args(["-bufsize"])
        .arg((bitrate * 2).to_string())
        .args(["-c:a", "aac", "-b:a", "96k", "-movflags", "+faststart"])
        .arg(&pending)
        .stdout(std::process::Stdio::piped());
    let child = command.spawn()?;
    let result = super::encoding::wait(child, session.video.duration, progress);
    let output = tokio::select! { _ = token.cancelled() => { let _ = std::fs::remove_file(&pending); anyhow::bail!("已停止视频拆解") }, result = result => result? };
    if !output.status.success() || std::fs::metadata(&pending)?.len() > LIMIT {
        let _ = std::fs::remove_file(&pending);
        anyhow::bail!("视频无法压缩到模型输入范围，请使用更短的视频");
    }
    std::fs::rename(pending, &path)?;
    Ok(path)
}

fn copy_file(source: &Path, target: &Path, progress: &Reporter) -> Result<()> {
    use std::io::{Read, Write};
    let mut input = std::fs::File::open(source)?;
    let total = input.metadata()?.len();
    let mut output = std::fs::File::create(target)?;
    let mut buffer = vec![0; 1024 * 1024];
    let mut copied = 0;
    loop {
        let size = input.read(&mut buffer)?;
        if size == 0 {
            break;
        }
        output.write_all(&buffer[..size])?;
        copied += size as u64;
        progress.send("copying", Some(copied), Some(total));
    }
    Ok(())
}
