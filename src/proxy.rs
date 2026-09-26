//! Preview-only files. Export always reads Asset.path, never this cache.
use crate::{
    media::{binary, probe, run},
    model::Asset,
};
use anyhow::{Result, ensure};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
pub fn prepare(asset: &Asset, root: &Path) -> Result<PathBuf> {
    ensure!(asset.kind == "video", "只有视频需要代理");
    ensure!(
        asset
            .id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_'),
        "素材 ID 无效"
    );
    let dir = root.join("proxies");
    std::fs::create_dir_all(&dir)?;
    // Versioned cache also changes when the source file changes.
    let meta = std::fs::metadata(&asset.path)?;
    let modified = meta
        .modified()?
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let output = dir.join(format!("{}-v1-{}-{modified}.mp4", asset.id, meta.len()));
    if output.is_file() {
        return Ok(output);
    }
    let partial = output.with_extension("partial.mp4");
    let result = (|| -> Result<()> {
        run(Command::new(binary("ffmpeg"))
            .args(["-v", "error", "-nostdin", "-y", "-i"])
            .arg(&asset.path)
            .args([
                "-map",
                "0:v:0",
                "-map",
                "0:a:0?",
                "-vf",
                "scale=640:640:force_original_aspect_ratio=decrease:force_divisible_by=2,setsar=1",
                "-c:v",
                "libx264",
                "-preset",
                "veryfast",
                "-crf",
                "23",
                "-pix_fmt",
                "yuv420p",
                "-g",
                "15",
                "-keyint_min",
                "15",
                "-sc_threshold",
                "0",
                "-bf",
                "0",
                "-fps_mode",
                "vfr",
                "-threads",
                "2",
                "-c:a",
                "aac",
                "-b:a",
                "128k",
                "-movflags",
                "+faststart",
            ])
            .arg(&partial))?;
        let metadata = probe(&partial)?;
        let duration = metadata["format"]["duration"]
            .as_str()
            .and_then(|v| v.parse::<f64>().ok())
            .unwrap_or(0.0);
        ensure!(
            (duration - asset.duration).abs() < 0.25,
            "代理时长与原片不匹配"
        );
        std::fs::rename(&partial, &output)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&partial);
    }
    result?;
    Ok(output)
}
