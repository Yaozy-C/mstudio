use crate::{
    media::{binary, run},
    model::{Asset, Clip, RenderSpec},
    render::tempo,
};
use anyhow::{Context, Result, ensure};
use std::{collections::HashMap, path::Path, process::Command};
/// Shared envelope and timing for the preview mix and final composition.
pub fn clip_filter(clip: &Clip, input: usize, label: &str) -> String {
    let start = clip.start;
    let fade_in = clip.fade_in.unwrap_or(0.).min(clip.duration());
    let fade_out = clip.fade_out.unwrap_or(0.).min(clip.duration());
    let fade_start = (clip.duration() - fade_out).max(0.);
    format!(
        "[{input}:a]asetpts=PTS-STARTPTS,{},aresample=48000,volume={},afade=t=in:d={fade_in},afade=t=out:st={fade_start}:d={fade_out},adelay={}S:all=1[{label}]",
        tempo(clip.speed),
        clip.volume,
        (start * 48000.).round() as u64
    )
}
pub fn preview(spec: &RenderSpec, assets: &[Asset], output: &Path) -> Result<bool> {
    let index: HashMap<_, _> = assets.iter().map(|a| (a.id.as_str(), a)).collect();
    let tracks: HashMap<_, _> = spec.tracks.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut total: f64 = 0.;
    let mut selected = vec![];
    for clip in &spec.clips {
        let asset = index.get(clip.asset_id.as_str()).context("预览素材丢失")?;
        clip.validate(asset)?;
        total = total.max(clip.start + clip.duration());
        let track = tracks.get(clip.track_id.as_str()).context("预览轨道丢失")?;
        if asset.has_audio && !track.muted && clip.volume > 0. {
            selected.push((clip, *asset));
        }
    }
    for caption in &spec.captions {
        ensure!(caption.end.is_finite() && caption.end >= 0., "字幕时间无效");
        total = total.max(caption.end);
    }
    ensure!(total <= 86400., "成片过长");
    if selected.is_empty() {
        return Ok(false);
    }
    let mut cmd = Command::new(binary("ffmpeg"));
    cmd.args(["-v", "error", "-y", "-filter_complex_threads", "2"]);
    let mut filters = vec![];
    let mut labels = String::new();
    for (i, (clip, asset)) in selected.iter().enumerate() {
        cmd.args([
            "-threads",
            "1",
            "-ss",
            &clip.trim_in.to_string(),
            "-t",
            &(clip.trim_out - clip.trim_in).to_string(),
            "-i",
            &asset.path,
        ]);
        filters.push(clip_filter(clip, i, &format!("a{i}")));
        labels.push_str(&format!("[a{i}]"));
    }
    filters.push(format!("{labels}amix=inputs={}:duration=longest:normalize=0,alimiter=limit=0.95:latency=1,apad[mix]",selected.len()));
    cmd.args([
        "-filter_complex",
        &filters.join(";"),
        "-map",
        "[mix]",
        "-vn",
        "-t",
        &(total + 0.1).to_string(),
        "-c:a",
        "aac",
        "-b:a",
        "192k",
        "-ar",
        "48000",
        "-ac",
        "2",
        "-movflags",
        "+faststart",
    ])
    .arg(output);
    run(&mut cmd)?;
    Ok(true)
}
