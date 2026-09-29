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
        .context("Transition frame requires the incoming clipId")?;
    let time = args["time"]
        .as_f64()
        .context("Transition frame requires time in seconds from transition start")?;
    let mut spec: RenderSpec = serde_json::from_value(doc.clone())?;
    let right = spec
        .clips
        .iter()
        .find(|c| c.id == id)
        .context("Transition target not found")?;
    ensure!(
        args["assetId"] == right.asset_id,
        "assetId must reference the incoming clip asset"
    );
    let transition = right
        .transition
        .as_ref()
        .context("Clip has no transition")?;
    ensure!(
        time.is_finite() && time >= 0. && time < transition.duration,
        "Frame time outside transition range"
    );
    let left = spec
        .clips
        .iter()
        .find(|c| c.id == transition.from_clip_id)
        .context("Outgoing clip not found")?;
    let ids = [left.asset_id.clone(), right.asset_id.clone()];
    let mut assets = store.assets()?;
    let root = store.media_root().join("assets").canonicalize()?;
    for id in &ids {
        ensure!(
            doc["assets"]
                .as_array()
                .is_some_and(|a| a.iter().any(|a| a["id"] == *id)),
            "Transition assets are outside this project"
        );
        let a = assets
            .iter()
            .find(|a| a.id == *id)
            .context("Transition asset missing")?;
        ensure!(
            Path::new(&a.path).canonicalize()?.starts_with(&root),
            "Asset path is outside the application media library"
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
    let path = paths
        .first()
        .context("Join is no longer valid; inspect clip positions")?;
    crate::project_storage::track_file(store, project, path)?;
    super::video_frame::frame(
        path,
        time,
        "",
        format!("transition:{id}@{time:.6}s/composited-no-overlays-no-audio"),
    )
}
