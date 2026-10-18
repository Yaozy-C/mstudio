use super::Reference;
use anyhow::{Context, Result, ensure};
use mstudio::{media, model::Asset};
use std::path::Path;

type Source = (Asset, Reference);
const MIN_VIDEO_SECONDS: f64 = 2.;

fn seconds((asset, reference): &Source) -> f64 {
    reference.end.unwrap_or(asset.duration) - reference.start.unwrap_or(0.)
}

pub(super) fn plan(sources: Vec<Source>) -> Result<Vec<Vec<Source>>> {
    let merge = sources
        .iter()
        .any(|s| s.0.kind == "video" && seconds(s) < MIN_VIDEO_SECONDS);
    if !merge {
        return Ok(sources.into_iter().map(|s| vec![s]).collect());
    }
    let duration: f64 = sources
        .iter()
        .filter(|s| s.0.kind == "video")
        .map(seconds)
        .sum();
    ensure!(
        duration >= MIN_VIDEO_SECONDS,
        "参考视频合并后不足 2 秒，请添加视频片段"
    );
    let mut groups: Vec<Vec<Source>> = Vec::new();
    let mut video_group = None;
    for source in sources {
        if source.0.kind == "video" {
            let index = *video_group.get_or_insert_with(|| {
                groups.push(Vec::new());
                groups.len() - 1
            });
            groups[index].push(source);
        } else {
            groups.push(vec![source]);
        }
    }
    Ok(groups)
}

pub(super) fn prepare_group(sources: &[Source], work: &Path) -> Result<(Vec<u8>, &'static str)> {
    let (asset, reference) = sources.first().context("请选择参考素材")?;
    if sources.len() == 1 {
        return prepare(asset, reference, work);
    }
    let width = (asset.width / 2 * 2).max(2);
    let height = (asset.height / 2 * 2).max(2);
    let audio = sources.iter().any(|s| s.0.has_audio);
    let output = work.join("reference.mp4");
    let mut cmd = media::command("ffmpeg");
    cmd.args(["-v", "error", "-y", "-filter_complex_threads", "1"]);
    for (asset, reference) in sources {
        cmd.args([
            "-ss",
            &reference.start.unwrap_or(0.).to_string(),
            "-t",
            &(reference.end.unwrap_or(asset.duration) - reference.start.unwrap_or(0.)).to_string(),
            "-i",
            &asset.path,
        ]);
    }
    let mut filters = Vec::new();
    let mut streams = String::new();
    for (index, source) in sources.iter().enumerate() {
        filters.push(format!(
            "[{index}:v:0]setpts=PTS-STARTPTS,scale={width}:{height}:force_original_aspect_ratio=decrease,pad={width}:{height}:(ow-iw)/2:(oh-ih)/2,setsar=1,fps=30[v{index}]"
        ));
        streams.push_str(&format!("[v{index}]"));
        if audio {
            let duration = seconds(source);
            filters.push(if source.0.has_audio {
                format!("[{index}:a:0]asetpts=PTS-STARTPTS,aresample=48000,aformat=sample_fmts=fltp:channel_layouts=stereo,apad,atrim=duration={duration}[a{index}]")
            } else {
                format!("anullsrc=r=48000:cl=stereo,atrim=duration={duration},asetpts=PTS-STARTPTS[a{index}]")
            });
            streams.push_str(&format!("[a{index}]"));
        }
    }
    filters.push(format!(
        "{streams}concat=n={}:v=1:a={}[v]{}",
        sources.len(),
        u8::from(audio),
        if audio { "[a]" } else { "" }
    ));
    cmd.args(["-filter_complex", &filters.join(";"), "-map", "[v]"]);
    if audio {
        cmd.args(["-map", "[a]", "-c:a", "aac"]);
    }
    cmd.args([
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
        "-movflags",
        "+faststart",
    ]);
    media::run(cmd.arg(&output))?;
    // Check the encoded video track itself, not an audio-padded container duration.
    let metadata = media::probe(&output)?;
    let duration = metadata["streams"]
        .as_array()
        .and_then(|streams| streams.iter().find(|s| s["codec_type"] == "video"))
        .and_then(|s| s["duration"].as_str())
        .and_then(|s| s.parse::<f64>().ok())
        .context("无法读取合并参考视频时长")?;
    ensure!(
        duration >= MIN_VIDEO_SECONDS,
        "参考视频合并后不足 2 秒，请添加视频片段"
    );
    Ok((std::fs::read(output)?, "video/mp4"))
}

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

#[cfg(test)]
#[path = "reference_prepare_tests.rs"]
mod tests;
