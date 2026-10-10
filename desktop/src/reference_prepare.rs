use super::Reference;
use anyhow::{Result, ensure};
use mstudio::{media, model::Asset};
use std::path::Path;

pub(super) fn prepare(
    asset: &Asset,
    reference: &Reference,
    work: &Path,
) -> Result<(Vec<u8>, &'static str)> {
    let image = asset.kind == "image";
    ensure!(image || asset.kind == "video", "请选择图片或视频参考");
    let output = work.join(if image {
        "reference.png"
    } else {
        "reference.mp4"
    });
    let mut cmd = media::command("ffmpeg");
    cmd.args(["-v", "error", "-y", "-threads", "2"]);
    if !image {
        let start = reference.start.unwrap_or(0.);
        let end = reference.end.unwrap_or(asset.duration);
        ensure!(
            start.is_finite()
                && end.is_finite()
                && start >= 0.
                && end <= asset.duration
                && end > start,
            "参考区间须有效且不能超出原视频"
        );
        cmd.args(["-ss", &start.to_string(), "-t", &(end - start).to_string()]);
    }
    cmd.args(["-i", &asset.path]);
    if image {
        cmd.args(["-frames:v", "1", "-update", "1"]);
    } else {
        cmd.args([
            "-vf",
            "scale=trunc(iw/2)*2:trunc(ih/2)*2",
            "-c:v",
            "libx264",
            "-threads",
            "2",
            "-preset",
            "fast",
            "-crf",
            "20",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-movflags",
            "+faststart",
        ]);
    }
    media::run(cmd.arg(&output))?;
    Ok((
        std::fs::read(output)?,
        if image { "image/png" } else { "video/mp4" },
    ))
}
