//! Encode Canvas-rendered RGBA frames once for native preview and final composition.
use crate::media::{binary, run};
use anyhow::{Result, ensure};
use std::{
    path::{Path, PathBuf},
    process::Command,
};
pub fn encode(
    frames: &[(PathBuf, f64)],
    fps: u32,
    duration: f64,
    repeat: bool,
    work: &Path,
    output: &Path,
) -> Result<()> {
    ensure!([24, 25, 30, 60].contains(&fps), "字幕帧率无效");
    ensure!(
        duration.is_finite() && duration > 0. && duration <= 86400.,
        "字幕时长无效"
    );
    ensure!(
        !frames.is_empty() && frames.len() <= 2002,
        "字幕动画帧数无效"
    );
    let mut manifest = String::from("ffconcat version 1.0\n");
    let mut total = 0.;
    // Use controlled relative names; never put user paths into concat syntax.
    for (i, (path, span)) in frames.iter().enumerate() {
        ensure!(
            span.is_finite() && *span > 0. && *span <= 86400.,
            "字幕帧时长无效"
        );
        let name = format!("frame-{i}.png");
        std::fs::copy(path, work.join(&name))?;
        manifest.push_str(&format!(
            "file '{name}'\noption framerate {fps}\nduration {span:.9}\n"
        ));
        total += span;
    }
    ensure!(total <= 86400. + 1., "字幕动画过长");
    manifest.push_str(&format!(
        "file 'frame-{}.png'\noption framerate {fps}\n",
        frames.len() - 1
    ));
    let list = work.join("frames.txt");
    std::fs::write(&list, manifest)?;
    let cycle = work.join("cycle.mov");
    run(Command::new(binary("ffmpeg"))
        .args(["-v", "error", "-y", "-f", "concat", "-safe", "0", "-i"])
        .arg(list)
        .args([
            "-vf",
            &format!("fps={fps}"),
            "-frames:v",
            &((total * fps as f64).round() as u64).to_string(),
            "-an",
            "-c:v",
            "png",
            "-pix_fmt",
            "rgba",
        ])
        .arg(&cycle))?;
    let mut command = Command::new(binary("ffmpeg"));
    command.args(["-v", "error", "-y"]);
    if repeat {
        command.args(["-stream_loop", "-1"]);
    }
    command.arg("-i").arg(cycle);
    if repeat {
        command.args(["-c:v", "copy"]);
    } else {
        command.args([
            "-vf",
            &format!("tpad=stop_mode=clone:stop_duration={duration}"),
            "-c:v",
            "png",
            "-pix_fmt",
            "rgba",
        ]);
    }
    run(command
        .args(["-t", &duration.to_string(), "-an"])
        .arg(output))
}
