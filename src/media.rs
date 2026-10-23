use crate::model::Asset;
use anyhow::{Context, Result, ensure};
use serde_json::Value;
use std::{
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

pub fn id() -> String {
    use std::sync::atomic::{AtomicU64, Ordering};
    static SEQ: AtomicU64 = AtomicU64::new(0);
    format!(
        "{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        SEQ.fetch_add(1, Ordering::Relaxed)
    )
}
pub fn binary(name: &str) -> PathBuf {
    if let Ok(exe) = std::env::current_exe()
        && let Some(path) = bundled_binary(&exe, name)
    {
        // Never silently use a machine-installed tool in a packaged app.
        // A missing bundled tool should surface as a packaging error.
        return path;
    }
    for prefix in ["/opt/homebrew/bin", "/usr/local/bin", "/usr/bin"] {
        let p = Path::new(prefix).join(name);
        if p.exists() {
            return p;
        }
    }
    PathBuf::from(name)
}
fn bundled_binary(exe: &Path, name: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    {
        let directory = exe.parent()?;
        // The manifest distinguishes a packaged app from cargo's build outputs.
        if directory.join("gstreamer/runtime.json").is_file() {
            return Some(directory.join("media").join(format!("{name}.exe")));
        }
    }
    let macos = exe.parent()?;
    let contents = macos.parent()?;
    if macos.file_name()? != "MacOS" || contents.file_name()? != "Contents" {
        return None;
    }
    Some(contents.join("Resources/gstreamer/bin").join(name))
}

/// Hide media subprocess console windows while retaining redirected output.
pub fn command(name: &str) -> Command {
    let mut command = Command::new(binary(name));
    quiet(&mut command);
    command
}
pub fn quiet(command: &mut Command) {
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    #[cfg(not(windows))]
    let _ = command;
}

pub fn run(command: &mut Command) -> Result<()> {
    quiet(command);
    let output = command.output().context("无法启动 FFmpeg；请安装 ffmpeg")?;
    ensure!(
        output.status.success(),
        "媒体处理失败：{}",
        String::from_utf8_lossy(&output.stderr)
            .chars()
            .rev()
            .take(1400)
            .collect::<String>()
            .chars()
            .rev()
            .collect::<String>()
    );
    Ok(())
}
pub fn probe(path: &Path) -> Result<Value> {
    let result = command("ffprobe")
        .args([
            "-v",
            "error",
            "-show_format",
            "-show_streams",
            "-of",
            "json",
        ])
        .arg(path)
        .output()
        .context("找不到 ffprobe；请安装 ffmpeg")?;
    ensure!(result.status.success(), "无法读取媒体文件");
    Ok(serde_json::from_slice(&result.stdout)?)
}
pub fn import(path: &Path, root: &Path) -> Result<Asset> {
    ensure!(path.is_file(), "素材文件不存在");
    if crate::text_asset::supported(path) {
        return crate::text_asset::import(path, root);
    }
    let metadata = probe(path)?;
    let streams = metadata["streams"]
        .as_array()
        .context("素材没有可读取的轨道")?;
    let video = streams
        .iter()
        .find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1);
    let has_audio = streams.iter().any(|s| s["codec_type"] == "audio");
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .context("素材文件没有有效扩展名")?
        .to_ascii_lowercase();
    let kind =
        if ["png", "jpg", "jpeg", "webp", "heic", "bmp", "tiff", "gif"].contains(&ext.as_str()) {
            "image"
        } else if video.is_some() {
            "video"
        } else {
            "audio"
        };
    ensure!(video.is_some() || has_audio, "不支持的媒体格式");
    ensure!(kind != "image" || video.is_some(), "图片没有可读取的画面");
    let duration = import_duration(kind, &metadata)?;
    let (width, height) = match video {
        Some(stream) => picture_dimensions(stream)?,
        None => (0, 0),
    };
    let id = id();
    std::fs::create_dir_all(root.join("assets"))?;
    std::fs::create_dir_all(root.join("previews"))?;
    let destination = root.join("assets").join(format!("{id}.{ext}"));
    std::fs::copy(path, &destination)?;
    let preview = root.join("previews").join(format!("{id}.jpg"));
    if video.is_some() {
        run(command("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(&destination)
            .args([
                "-frames:v",
                "1",
                "-vf",
                "scale=480:480:force_original_aspect_ratio=decrease",
                "-update",
                "1",
            ])
            .arg(&preview))?;
    }
    Ok(Asset {
        missing: false,
        attachment_only: false,
        generated: false,
        id,
        name: path
            .file_name()
            .context("素材文件名无效")?
            .to_string_lossy()
            .into(),
        kind: kind.into(),
        path: destination.to_string_lossy().into(),
        preview: if preview.exists() {
            preview.to_string_lossy().into()
        } else {
            String::new()
        },
        duration,
        width,
        height,
        has_audio,
    })
}

fn import_duration(kind: &str, metadata: &Value) -> Result<f64> {
    // A still image has a display duration; audio/video require a measured duration.
    if kind == "image" {
        return Ok(5.0);
    }
    let duration: f64 = metadata["format"]["duration"]
        .as_str()
        .context("无法读取音视频时长")?
        .parse()
        .context("音视频时长格式无效")?;
    ensure!(duration.is_finite() && duration > 0., "音视频时长无效");
    Ok(duration)
}

pub fn picture_dimensions(stream: &Value) -> Result<(u32, u32)> {
    let width = u32::try_from(stream["width"].as_u64().context("无法读取画面宽度")?)?;
    let height = u32::try_from(stream["height"].as_u64().context("无法读取画面高度")?)?;
    ensure!(width > 0 && height > 0, "画面宽高必须大于零");
    Ok((width, height))
}

#[cfg(test)]
#[path = "media_import_tests.rs"]
mod import_tests;

#[cfg(test)]
mod binary_tests {
    use super::*;

    #[test]
    fn relocated_app_uses_private_tools_even_when_missing() {
        let exe = Path::new("/Applications/视频 工作室.app/Contents/MacOS/mstudio-desktop");
        for name in ["ffmpeg", "ffprobe"] {
            assert_eq!(
                bundled_binary(exe, name),
                Some(
                    Path::new("/Applications/视频 工作室.app/Contents/Resources/gstreamer/bin")
                        .join(name)
                )
            );
        }
        assert_eq!(
            bundled_binary(Path::new("/tmp/target/release/example"), "ffmpeg"),
            None
        );
    }
}
