//! Inspect the same cached transition patch used by native playback, without importing media.
use crate::database::Store;
use anyhow::{Context, Result, ensure};
use mstudio::model::RenderSpec;
use serde_json::Value;
use std::path::Path;

pub fn parts(
    store: &Store,
    project: &str,
    doc: &Value,
    args: &Value,
) -> Result<(String, Vec<Value>)> {
    let id = args["clipId"]
        .as_str()
        .context("转场抽帧需要后片段 clipId")?;
    let time = args["time"]
        .as_f64()
        .context("转场抽帧需要 time（转场开始后的秒数）")?;
    let mut spec: RenderSpec = serde_json::from_value(doc.clone())?;
    let right = spec
        .clips
        .iter()
        .find(|c| c.id == id)
        .context("转场目标不存在")?;
    ensure!(
        args["assetId"] == right.asset_id,
        "assetId 必须是后片段的素材 ID"
    );
    let transition = right.transition.as_ref().context("此片段没有转场")?;
    ensure!(
        time.is_finite() && time >= 0. && time < transition.duration,
        "抽帧时间超出转场范围"
    );
    let left = spec
        .clips
        .iter()
        .find(|c| c.id == transition.from_clip_id)
        .context("前片段不存在")?;
    let ids = [left.asset_id.clone(), right.asset_id.clone()];
    let mut assets = store.assets()?;
    let root = store.media_root().join("assets").canonicalize()?;
    for id in &ids {
        ensure!(
            doc["assets"]
                .as_array()
                .is_some_and(|a| a.iter().any(|a| a["id"] == *id)),
            "转场素材不属于当前工程"
        );
        let a = assets
            .iter()
            .find(|a| a.id == *id)
            .context("转场素材丢失")?;
        ensure!(
            Path::new(&a.path).canonicalize()?.starts_with(&root),
            "素材路径不属于应用素材库"
        );
    }
    // Preserve all clips for overlap validation, but render only the requested seam.
    for clip in &mut spec.clips {
        if clip.id != id {
            clip.transition = None;
        }
    }
    let paths = mstudio::transitions::prepare(
        &mut spec,
        &mut assets,
        &store.media_root().join("proxies"),
        true,
    )?;
    let path = paths.first().context("接缝已失效，请重新检查片段位置")?;
    crate::project_storage::track_file(store, project, path)?;
    super::video_frame::frame(
        path,
        time,
        "",
        format!("transition:{id}@{time:.6}s/composited-no-overlays-no-audio"),
    )
}
