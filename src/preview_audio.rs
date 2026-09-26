//! Pitch-preserving speed audio is cached once, then played by the same MLT graph.
use crate::{
    media::{binary, run},
    model::{Asset, RenderSpec, Track},
    render::tempo,
};
use anyhow::{Context, Result};
use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
    process::Command,
};
pub fn prepare(
    spec: &mut RenderSpec,
    assets: &mut Vec<Asset>,
    root: &Path,
) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut extra = Vec::new();
    let mut track_id = "__mlt_speed_audio".to_string();
    while spec.tracks.iter().any(|t| t.id == track_id) {
        track_id.push('_');
    }
    for (i, clip) in spec.clips.iter_mut().enumerate() {
        let asset = assets
            .iter()
            .find(|a| a.id == clip.asset_id)
            .context("预览素材缺失")?
            .clone();
        clip.validate(&asset)?;
        let muted = spec
            .tracks
            .iter()
            .find(|t| t.id == clip.track_id)
            .is_some_and(|t| t.muted);
        if (clip.speed - 1.).abs() < 0.00001 || !asset.has_audio || muted || clip.volume == 0. {
            continue;
        }
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        asset.path.hash(&mut hash);
        let metadata = std::fs::metadata(&asset.path)?;
        metadata.len().hash(&mut hash);
        metadata.modified()?.hash(&mut hash);
        for v in [clip.trim_in, clip.trim_out, clip.speed] {
            v.to_bits().hash(&mut hash);
        }
        std::fs::create_dir_all(root)?;
        let path = root.join(format!("mlt-speed-audio-v1-{:x}.wav", hash.finish()));
        if !path.is_file() {
            let partial = root.join(format!("mlt-speed-{}.wav", crate::media::id()));
            let result = run(Command::new(binary("ffmpeg"))
                .args([
                    "-v",
                    "error",
                    "-nostdin",
                    "-y",
                    "-ss",
                    &clip.trim_in.to_string(),
                    "-t",
                    &(clip.trim_out - clip.trim_in).to_string(),
                    "-i",
                ])
                .arg(&asset.path)
                .args([
                    "-vn",
                    "-af",
                    &tempo(clip.speed),
                    "-ar",
                    "48000",
                    "-ac",
                    "2",
                    "-c:a",
                    "pcm_s16le",
                ])
                .arg(&partial));
            if let Err(error) = result {
                let _ = std::fs::remove_file(&partial);
                return Err(error);
            }
            std::fs::rename(partial, &path)?;
        }
        let duration = clip.duration();
        let mut id = format!("__mlt_speed_{i}");
        while assets.iter().any(|a| a.id == id) {
            id.push('_');
        }
        let mut audio = clip.clone();
        audio.id = id.clone();
        audio.asset_id = id.clone();
        audio.track_id = track_id.clone();
        audio.trim_in = 0.;
        audio.trim_out = duration;
        audio.speed = 1.;
        // The picture producer remains a timewarp; its audio is replaced, not doubled.
        clip.volume = 0.;
        assets.push(Asset {
            id,
            name: asset.name,
            kind: "audio".into(),
            path: path.to_string_lossy().into(),
            preview: String::new(),
            duration,
            width: 0,
            height: 0,
            has_audio: true,
            missing: false,
            generated: true,
        });
        extra.push(audio);
        files.push(path);
    }
    if !extra.is_empty() {
        spec.tracks.push(Track {
            id: track_id,
            kind: "audio".into(),
            muted: false,
            hidden: false,
        });
        spec.clips.extend(extra);
    }
    Ok(files)
}
