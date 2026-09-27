//! Cached cut-centred transitions; edit timing and audio remain unchanged.
use crate::model::{Asset, Clip, RenderSpec};
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    hash::{Hash, Hasher},
    path::{Path, PathBuf},
};
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Transition {
    pub from_clip_id: String,
    pub kind: String,
    pub duration: f64,
}
pub const KINDS: &[&str] = &[
    "fade",
    "fadeblack",
    "fadewhite",
    "wipeleft",
    "wiperight",
    "slideleft",
    "slideright",
    "smoothleft",
    "smoothright",
    "circleopen",
    "circleclose",
    "dissolve",
];
impl Transition {
    pub fn validate(&self) -> Result<()> {
        ensure!(KINDS.contains(&self.kind.as_str()), "不支持的转场类型");
        ensure!(
            self.duration.is_finite() && (0.05..=3.).contains(&self.duration),
            "转场时长为 0.05–3 秒"
        );
        Ok(())
    }
}
pub fn full_frame(c: &Clip) -> bool {
    c.x.unwrap_or(0.5) == 0.5
        && c.y.unwrap_or(0.5) == 0.5
        && c.scale.unwrap_or(1.) == 1.
        && c.opacity.unwrap_or(1.) == 1.
}
pub fn prepare(
    spec: &mut RenderSpec,
    assets: &mut Vec<Asset>,
    root: &Path,
    preview: bool,
) -> Result<Vec<PathBuf>> {
    ensure!(
        [24, 25, 30, 60].contains(&spec.fps)
            && (64..=3840).contains(&spec.width)
            && (64..=3840).contains(&spec.height),
        "转场输出尺寸或帧率无效"
    );
    let original = spec.clips.clone();
    for c in &mut spec.clips {
        c.transition = None;
    }
    let mut files = vec![];
    let base = spec
        .tracks
        .iter()
        .find(|t| t.kind == "video")
        .map(|t| t.id.as_str());
    for right in &original {
        let Some(t) = &right.transition else { continue };
        t.validate()?;
        let Some(left) = original.iter().find(|c| c.id == t.from_clip_id) else {
            continue;
        };
        // A moved/deleted neighbour turns the saved seam back into a cut.
        if left.track_id != right.track_id
            || (left.start + left.duration() - right.start).abs() > 0.5 / spec.fps as f64
        {
            continue;
        }
        ensure!(
            Some(right.track_id.as_str()) == base && full_frame(left) && full_frame(right),
            "转场目前支持底层画面轨的全幅不透明片段，请先恢复画面位置与缩放"
        );
        ensure!(
            t.duration <= left.duration().min(right.duration()) + 1e-6,
            "转场不能超过相邻片段时长"
        );
        ensure!(
            !original.iter().any(|c| c.track_id == right.track_id
                && c.id != left.id
                && c.id != right.id
                && c.start < right.start + t.duration / 2.
                && c.start + c.duration() > right.start - t.duration / 2.),
            "接缝有其他重叠片段，无法添加转场"
        );
        let a = assets
            .iter()
            .find(|a| a.id == left.asset_id)
            .context("转场素材缺失")?
            .clone();
        let b = assets
            .iter()
            .find(|a| a.id == right.asset_id)
            .context("转场素材缺失")?
            .clone();
        left.validate(&a)?;
        right.validate(&b)?;
        let ratio = if preview {
            (640. / spec.width.max(spec.height) as f64).min(1.)
        } else {
            1.
        };
        let w = ((spec.width as f64 * ratio / 2.).round() as u32 * 2).max(2);
        let h = ((spec.height as f64 * ratio / 2.).round() as u32 * 2).max(2);
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        ("transition-v1", w, h, spec.fps).hash(&mut hash);
        for (c, a) in [(left, &a), (right, &b)] {
            serde_json::to_string(c)?.hash(&mut hash);
            a.path.hash(&mut hash);
            let m = std::fs::metadata(&a.path)?;
            m.len().hash(&mut hash);
            m.modified()?.hash(&mut hash);
        }
        std::fs::create_dir_all(root)?;
        let path = root.join(format!("transition-{:x}.mp4", hash.finish()));
        if !path.is_file() {
            crate::transition_render::render(left, right, &a, &b, t, (w, h, spec.fps), &path)?;
        }
        let mut id = format!("__transition_{}", right.id);
        while assets.iter().any(|a| a.id == id) || spec.clips.iter().any(|c| c.id == id) {
            id.push('_');
        }
        assets.push(Asset {
            id: id.clone(),
            name: "转场预览".into(),
            kind: "video".into(),
            path: path.to_string_lossy().into(),
            preview: String::new(),
            duration: t.duration,
            width: w,
            height: h,
            has_audio: false,
            missing: false,
            generated: true,
        });
        spec.clips.push(Clip {
            id: id.clone(),
            asset_id: id,
            trim_in: 0.,
            trim_out: t.duration,
            speed: 1.,
            volume: 0.,
            start: right.start - t.duration / 2.,
            track_id: right.track_id.clone(),
            ..Default::default()
        });
        files.push(path);
    }
    Ok(files)
}
