use super::render::Export;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::collections::HashMap;

/// Update shot references only. Submitted tasks and timeline clips keep their snapshots.
pub(super) fn update(document: &mut Value, previous: &[Export], asset: &Value) -> Result<()> {
    let old: HashMap<String, f64> = previous
        .iter()
        .map(|video| (video.asset.id.clone(), video.asset.duration))
        .collect();
    let new_id = asset["id"].as_str().context("预演素材 ID 缺失")?;
    let duration = asset["duration"].as_f64().context("预演时长缺失")?;
    for node in document["nodes"].as_array_mut().context("镜头目录无效")? {
        if node["shot"].is_null() {
            continue;
        }
        let Some(refs) = node["references"].as_array_mut() else {
            continue;
        };
        for reference in refs.iter_mut() {
            let Some(old_duration) = reference["assetId"].as_str().and_then(|id| old.get(id))
            else {
                continue;
            };
            let start = reference["start"].as_f64().unwrap_or(0.);
            let end = reference["end"].as_f64().unwrap_or(*old_duration);
            let full = start.abs() < 0.001 && (end - old_duration).abs() < 0.05;
            reference["assetId"] = json!(new_id);
            if full || start >= duration {
                reference["start"] = json!(0.);
                reference["end"] = json!(duration);
            } else {
                reference["start"] = json!(start.max(0.));
                reference["end"] = json!(end.min(duration).max(start + 0.001).min(duration));
            }
        }
        let mut seen_new = false;
        refs.retain(|reference| {
            if reference["assetId"] != new_id {
                return true;
            }
            let keep = !seen_new;
            seen_new = true;
            keep
        });
    }
    Ok(())
}
