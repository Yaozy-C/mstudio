//! GES inputs. Cache unsupported visual/time filters using the export definitions;
//! GES still owns clip scheduling, compositing, the audio clock and seeking.
use crate::{
    media::{binary, run},
    model::{Asset, RenderSpec},
};
use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    process::Command,
};
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Layer {
    pub path: String,
    pub start: f64,
    pub duration: f64,
    pub trim: f64,
    pub x: i32,
    pub y: i32,
    pub width: i32,
    pub height: i32,
    pub opacity: f64,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Plan {
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub duration: f64,
    pub layers: Vec<Layer>,
    pub audio: Option<String>,
    pub files: Vec<PathBuf>,
}
fn source_hash(hash: &mut impl Hasher, a: &Asset) -> Result<()> {
    a.path.hash(hash);
    let m = std::fs::metadata(&a.path)?;
    m.len().hash(hash);
    m.modified()?.hash(hash);
    Ok(())
}
pub fn prepare(spec: &RenderSpec, assets: &[Asset], root: &Path, edge: u32) -> Result<Plan> {
    // Shared validator also checks caption assets, tracks, times and effects.
    crate::preview_validate::validate(spec, assets, edge)?;
    std::fs::create_dir_all(root)?;
    let ratio = edge as f64 / spec.width.max(spec.height) as f64;
    let width = ((spec.width as f64 * ratio / 2.).round() as u32 * 2).max(2);
    let height = ((spec.height as f64 * ratio / 2.).round() as u32 * 2).max(2);
    let duration = spec
        .clips
        .iter()
        .map(|c| c.start + c.duration())
        .chain(spec.captions.iter().map(|c| c.end))
        .fold(0., f64::max);
    let mut plan = Plan {
        width,
        height,
        fps: spec.fps,
        duration,
        layers: vec![],
        audio: None,
        files: vec![],
    };
    for t in &spec.tracks {
        if t.kind != "video" || t.hidden {
            continue;
        }
        for c in spec.clips.iter().filter(|c| c.track_id == t.id) {
            let a = assets
                .iter()
                .find(|a| a.id == c.asset_id)
                .context("GES 素材不存在")?;
            if a.kind == "audio" {
                continue;
            }
            let scale = c.scale.unwrap_or(1.);
            let fit = (width as f64 / a.width.max(1) as f64)
                .min(height as f64 / a.height.max(1) as f64)
                * scale;
            let w = (a.width as f64 * fit).round().max(2.) as i32;
            let h = (a.height as f64 * fit).round().max(2.) as i32;
            let filters = crate::visual::ffmpeg(c.visual.as_ref(), w as i64, h as i64)?;
            let needs_cache =
                !filters.is_empty() || (a.kind != "image" && (c.speed - 1.).abs() > 0.00001);
            let mut path = a.path.clone();
            let mut trim = if a.kind == "image" { 0. } else { c.trim_in };
            if needs_cache {
                let mut hash = std::collections::hash_map::DefaultHasher::new();
                ("ges-picture-v1", width, height, w, h, spec.fps).hash(&mut hash);
                source_hash(&mut hash, a)?;
                serde_json::to_string(&(&c.visual, c.trim_in, c.trim_out, c.speed))?
                    .hash(&mut hash);
                let image = a.kind == "image";
                let ext = if image { "png" } else { "mp4" };
                let cached = root.join(format!("ges-picture-{:x}.{ext}", hash.finish()));
                if !cached.is_file() {
                    let temporary = root.join(format!("ges-picture-{}.{ext}", crate::media::id()));
                    let mut cmd = Command::new(binary("ffmpeg"));
                    cmd.args(["-v", "error", "-nostdin", "-y", "-threads", "2"]);
                    if !image {
                        cmd.args([
                            "-ss",
                            &c.trim_in.to_string(),
                            "-t",
                            &(c.trim_out - c.trim_in).to_string(),
                        ]);
                    }
                    cmd.arg("-i").arg(&a.path).args(["-an", "-vf"]);
                    let mut vf = format!(
                        "scale={width}:{height}:force_original_aspect_ratio=decrease:force_divisible_by=2"
                    );
                    if !image {
                        vf = format!("setpts=(PTS-STARTPTS)/{},fps={},{}", c.speed, spec.fps, vf);
                    }
                    if !filters.is_empty() {
                        vf.push(',');
                        vf.push_str(&filters);
                    }
                    cmd.arg(vf);
                    if image {
                        cmd.args(["-frames:v", "1", "-pix_fmt", "rgba"]);
                    } else {
                        cmd.args([
                            "-c:v",
                            "libx264",
                            "-preset",
                            "veryfast",
                            "-crf",
                            "16",
                            "-pix_fmt",
                            "yuv420p",
                            "-threads",
                            "2",
                            "-t",
                            &c.duration().to_string(),
                        ]);
                    }
                    if let Err(e) = run(cmd.arg(&temporary)) {
                        let _ = std::fs::remove_file(&temporary);
                        return Err(e);
                    }
                    std::fs::rename(temporary, &cached)?;
                }
                path = cached.to_string_lossy().into();
                trim = 0.;
                plan.files.push(cached);
            }
            plan.layers.push(Layer {
                path,
                start: c.start,
                duration: c.duration(),
                trim,
                x: (c.x.unwrap_or(0.5) * width as f64 - w as f64 / 2.).round() as i32,
                y: (c.y.unwrap_or(0.5) * height as f64 - h as f64 / 2.).round() as i32,
                width: w,
                height: h,
                opacity: c.opacity.unwrap_or(1.),
            });
        }
    }
    for c in &spec.captions {
        let a = assets
            .iter()
            .find(|a| a.id == c.asset_id)
            .context("GES 字幕不存在")?;
        plan.layers.push(Layer {
            path: a.path.clone(),
            start: c.start,
            duration: c.end - c.start,
            trim: 0.,
            x: 0,
            y: 0,
            width: width as i32,
            height: height as i32,
            opacity: 1.,
        });
    }
    // Use the export mixer for pitch-preserving speed, fades, gain and limiting.
    // This is an audio-only cache inside GES, never an independent HTML clock.
    let mut hash = std::collections::hash_map::DefaultHasher::new();
    "ges-audio-v1".hash(&mut hash);
    serde_json::to_string(spec)?.hash(&mut hash);
    for c in &spec.clips {
        source_hash(
            &mut hash,
            assets
                .iter()
                .find(|a| a.id == c.asset_id)
                .context("GES 音频不存在")?,
        )?;
    }
    let path = root.join(format!("ges-audio-{:x}.m4a", hash.finish()));
    if path.is_file() {
        plan.audio = Some(path.to_string_lossy().into());
    } else {
        let temporary = root.join(format!("ges-audio-{}.m4a", crate::media::id()));
        match crate::audio_mix::preview(spec, assets, &temporary) {
            Ok(true) => {
                std::fs::rename(temporary, &path)?;
                plan.audio = Some(path.to_string_lossy().into());
            }
            Ok(false) => {}
            Err(e) => {
                let _ = std::fs::remove_file(temporary);
                return Err(e);
            }
        }
    }
    if plan.audio.is_some() {
        plan.files.push(path);
    }
    Ok(plan)
}
