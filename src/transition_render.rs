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
    let frames = (d * fps as f64).ceil() as u32;
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
    let circular = matches!(transition.kind.as_str(), "circleopen" | "circleclose");
    let format = if transition.design.is_some() || circular {
        "gbrp"
    } else {
        "yuv444p"
    };
    for (i, (clip, asset, center)) in [(left, a, left.trim_out), (right, b, right.trim_in)]
        .iter()
        .enumerate()
    {
        let source_start = center - half * clip.speed;
        let start = source_start.max(0.);
        let pad_start = (-source_start / clip.speed).max(0.);
        // Decode a short preroll before seeking into the tail. Accurate input
        // seeking at the very end can otherwise discard the last source frame,
        // leaving xfade with an empty input (a flash/cut instead of a transition).
        let seek = if asset.kind == "image" {
            0.
        } else {
            (start - 1.).max(0.)
        };
        let lead = if asset.kind == "image" {
            0.
        } else {
            (start - seek) / clip.speed
        };
        let available = (asset.duration - seek).max(1. / fps as f64);
        if asset.kind == "image" {
            cmd.args(["-loop", "1", "-framerate", &fps.to_string()]);
        }
        cmd.args([
            "-ss",
            &seek.to_string(),
            "-t",
            &((d + lead) * clip.speed)
                .min(if asset.kind == "image" {
                    d * clip.speed
                } else {
                    available
                })
                .to_string(),
            "-i",
        ])
        .arg(&asset.path);
        let grade = crate::visual::ffmpeg(clip.visual.as_ref(), w as i64, h as i64)?;
        let grade = if grade.is_empty() {
            grade
        } else {
            format!("{grade},")
        };
        let motion = transition
            .design
            .as_ref()
            .map(|design| design.motion_filter(i == 1, d, fps))
            .transpose()?
            .unwrap_or_default();
        filters.push(format!("[{i}:v]setpts=(PTS-STARTPTS)/{},fps={fps},scale={w}:{h}:force_original_aspect_ratio=decrease,setsar=1,{grade}pad={w}:{h}:(ow-iw)/2:(oh-ih)/2:color=black,format={format},tpad=stop_mode=clone:stop_duration={d},trim=start={lead},setpts=PTS-STARTPTS,tpad=start_mode=clone:start_duration={pad_start}:stop_mode=clone:stop_duration={d},trim=end_frame={frames},settb=AVTB,setpts=PTS-STARTPTS{motion}[v{i}]", clip.speed));
    }
    let expression = transition
        .design
        .as_ref()
        .map(|d| d.expression())
        .transpose()?
        .or_else(|| circular.then(|| circle_expression(&transition.kind, d, fps)));
    let kind = if circular { "custom" } else { &transition.kind };
    let custom = expression
        .map(|e| format!(":expr='{e}'"))
        .unwrap_or_default();
    filters.push(format!(
        "[v0][v1]xfade=transition={}:duration={d}:offset=0{custom},format=yuv420p[out]",
        kind
    ));
    let partial = output.with_file_name(format!("transition-partial-{}.mp4", crate::media::id()));
    let result = run(cmd
        .args([
            "-filter_complex",
            &filters.join(";"),
            "-map",
            "[out]",
            "-an",
            "-frames:v",
            &frames.to_string(),
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

/// A true pixel-space circle with a two-pixel antialiased edge. Normalize to
/// the last encoded frame, including very short transitions, so handing back
/// to the incoming clip never leaves a residual outgoing image.
fn circle_expression(kind: &str, duration: f64, fps: u32) -> String {
    let last = ((duration * fps as f64).ceil() - 1.).max(1.) / fps as f64;
    let progress = format!("clip((1-P)*{},0,1)", duration / last);
    let radius = if kind == "circleclose" {
        format!("(1-({progress}))")
    } else {
        progress
    };
    let mask = format!("clip(((hypot(W/2,H/2)+2)*({radius})-hypot(X-W/2,Y-H/2))/2,0,1)");
    if kind == "circleclose" {
        format!("A*({mask})+B*(1-({mask}))")
    } else {
        format!("A*(1-({mask}))+B*({mask})")
    }
}
