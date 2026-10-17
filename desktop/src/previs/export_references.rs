use super::render::Export;
use anyhow::{Context, Result};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};

/// Update shot references only. Submitted tasks and timeline clips keep their snapshots.
pub(super) fn update(document: &mut Value, previous: &[Export], asset: &Value) -> Result<()> {
    let old: HashMap<String, f64> = previous
        .iter()
        .filter_map(|video| {
            let id = video.shared_asset_id.as_ref()?;
            let duration = document["assets"]
                .as_array()
                .and_then(|assets| assets.iter().find(|a| a["id"] == *id))
                .and_then(|asset| asset["duration"].as_f64())
                .unwrap_or(video.asset.duration);
            Some((id.clone(), duration))
        })
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
    let mut usage = document.clone();
    usage["assets"] = json!([]);
    usage["removedAssetIds"] = json!([]);
    let mut used = HashSet::new();
    crate::project_storage::references(&usage, &mut used);
    let mut removed = Vec::new();
    document["assets"]
        .as_array_mut()
        .context("素材目录无效")?
        .retain_mut(|item| {
            let Some(id) = item["id"].as_str() else {
                return true;
            };
            if id == new_id || !old.contains_key(id) {
                return true;
            }
            if used.contains(id) {
                item["inLibrary"] = json!(false);
                true
            } else {
                removed.push(json!(id));
                false
            }
        });
    if !removed.is_empty() {
        if !document["removedAssetIds"].is_array() {
            document["removedAssetIds"] = json!([]);
        }
        document["removedAssetIds"]
            .as_array_mut()
            .unwrap()
            .extend(removed);
    }
    let assets = document["assets"].as_array_mut().unwrap();
    if !assets.iter().any(|a| a["id"] == new_id) {
        assets.push(asset.clone());
    }
    Ok(())
}
