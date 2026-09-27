use crate::{
    media::{binary, run},
    model::{Asset, Clip},
    transitions::Transition,
};
use anyhow::Result;
use std::{path::Path, process::Command};
pub fn render(
    left: &Clip,
    right: &Clip,
    a: &Asset,
    b: &Asset,
    transition: &Transition,
    dimensions: (u32, u32, u32),
    output: &Path,
) -> Result<()> {
    let (w, h, fps) = dimensions;
    let d = transition.duration;
    let half = d / 2.;
    let mut cmd = Command::new(binary("ffmpeg"));
    cmd.args([
        "-v",
        "error",
        "-nostdin",
        "-y",
        "-filter_complex_threads",
        "2",
    ]);
    let mut filters = vec![];
    for (i, (clip, asset, center)) in [(left, a, left.trim_out), (right, b, right.trim_in)]
        .iter()
        .enumerate()
    {
        let source_start = center - half * clip.speed;
        let start = source_start.max(0.);
        let pad_start = (-source_start / clip.speed).max(0.);
        let available = (asset.duration - start).max(1. / fps as f64);
        if asset.kind == "image" {
            cmd.args(["-loop", "1", "-framerate", &fps.to_string()]);
        }
        cmd.args([
            "-ss",
            &start.to_string(),
            "-t",
            &(d * clip.speed)
                .min(if asset.kind == "image" {
                    d * clip.speed
                } else {
                    available
                })
                .to_string(),
            "-i",
        ])
        .arg(&asset.path);
        let grade = crate::visual::ffmpeg(clip.visual.as_ref(), w as i64, h as i64);
        let grade = if grade.is_empty() {
            grade
        } else {
            format!("{grade},")
        };
        filters.push(format!("[{i}:v]setpts=(PTS-STARTPTS)/{},fps={fps},scale={w}:{h}:force_original_aspect_ratio=decrease,setsar=1,{grade}pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black,format=yuv444p,tpad=start_mode=clone:start_duration={pad_start}:stop_mode=clone:stop_duration={d},trim=duration={d},settb=AVTB,setpts=PTS-STARTPTS[v{i}]", clip.speed));
    }
    filters.push(format!(
        "[v0][v1]xfade=transition={}:duration={d}:offset=0,format=yuv420p[out]",
        transition.kind
    ));
    let partial = output.with_file_name(format!("transition-partial-{}.mp4", crate::media::id()));
    let result = run(cmd
        .args([
            "-filter_complex",
            &filters.join(";"),
            "-map",
            "[out]",
            "-an",
            "-t",
            &d.to_string(),
            "-c:v",
            "libx264",
            "-preset",
            "veryfast",
            "-crf",
            "16",
        ])
        .arg(&partial));
    if let Err(error) = result {
        let _ = std::fs::remove_file(&partial);
        return Err(error);
    }
    std::fs::rename(partial, output)?;
    Ok(())
}
