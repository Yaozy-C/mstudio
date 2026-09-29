//! Validate preview inputs before preparing any media caches.
use crate::model::{Asset, RenderSpec};
use anyhow::{Context, Result, ensure};
use std::collections::HashMap;
pub fn validate(spec: &RenderSpec, assets: &[Asset], edge: u32) -> Result<()> {
    ensure!([0, 640, 1280].contains(&edge), "预览清晰度无效");
    ensure!([24, 25, 30, 60].contains(&spec.fps), "不支持的预览帧率");
    ensure!(
        (64..=3840).contains(&spec.width) && (64..=3840).contains(&spec.height),
        "预览尺寸无效"
    );
    let assets: HashMap<_, _> = assets.iter().map(|a| (a.id.as_str(), a)).collect();
    let tracks: HashMap<_, _> = spec.tracks.iter().map(|t| (t.id.as_str(), t)).collect();
    ensure!(tracks.len() == spec.tracks.len(), "轨道 ID 重复");
    let mut total: f64 = 0.;
    for c in &spec.clips {
        let a = assets.get(c.asset_id.as_str()).context("预览素材不存在")?;
        ensure!(!a.missing, "预览素材缺失：{}", a.name);
        c.validate(a)?;
        let t = tracks.get(c.track_id.as_str()).context("预览轨道不存在")?;
        ensure!(t.kind == "video" || t.kind == "audio", "轨道类型无效");
        total = total.max(c.start + c.duration());
    }
    for c in &spec.captions {
        ensure!(
            c.start.is_finite()
                && c.end.is_finite()
                && c.start >= 0.
                && c.end > c.start
                && c.end <= 86400.,
            "字幕时间无效"
        );
        ensure!(
            assets
                .get(c.asset_id.as_str())
                .is_some_and(|a| a.kind == "image" && !a.missing),
            "字幕画面尚未准备"
        );
        total = total.max(c.end);
    }
    ensure!(total > 0. && total <= 86400., "预览时间线为空或过长");
    Ok(())
}
