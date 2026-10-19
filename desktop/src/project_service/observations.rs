//! DSH-style host-owned observations: scoped to a turn and an entity, never a
//! model-supplied global revision. Layout and execution telemetry are not content.
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
#[cfg(test)]
#[path = "observation_tests.rs"]
mod tests;
pub fn versions(document: &Value) -> Value {
    json!(
        targets(document)
            .into_iter()
            .map(|(key, value)| (key, version(&value)))
            .collect::<BTreeMap<_, _>>()
    )
}
fn canonical(value: &mut Value) {
    match value {
        Value::Object(fields) => {
            fields.sort_keys();
            for child in fields.values_mut() {
                canonical(child);
            }
        }
        Value::Array(items) => {
            for child in items {
                canonical(child);
            }
        }
        // JSON.stringify turns 5.0 into 5. Normalize integral floats for both
        // target comparisons and hashes, without rounding large integer IDs.
        Value::Number(number) if number.is_f64() => {
            let float = number.as_f64().unwrap();
            if float.fract() == 0.0 {
                if (0.0..u64::MAX as f64).contains(&float) {
                    *number = (float as u64).into();
                } else if float < 0.0 && float >= i64::MIN as f64 {
                    *number = (float as i64).into();
                }
            }
        }
        _ => {}
    }
}
fn version(value: &Value) -> String {
    let mut value = value.clone();
    canonical(&mut value);
    format!("{:x}", Sha256::digest(value.to_string()))
}
use std::collections::{BTreeMap, BTreeSet};
pub fn targets(document: &Value) -> BTreeMap<String, Value> {
    let mut result = BTreeMap::new();
    for (section, prefix) in [
        ("nodes", "node"),
        ("clips", "clip"),
        ("tracks", "track"),
        ("captions", "caption"),
        ("assets", "asset"),
        ("previsAssets", "asset"),
    ] {
        for item in document[section].as_array().into_iter().flatten() {
            if let Some(id) = item["id"].as_str() {
                let mut value = item.clone();
                if prefix == "node"
                    && let Some(obj) = value.as_object_mut()
                {
                    for key in ["x", "y", "width", "height"] {
                        obj.remove(key);
                    }
                }
                result.insert(format!("{prefix}:{id}"), value);
            }
        }
    }
    if let Some(tasks) = document["production"]["drafts"].as_object() {
        for (id, task) in tasks {
            let mut value = task.clone();
            if let Some(obj) = value.as_object_mut() {
                for key in [
                    "status",
                    "progress",
                    "error",
                    "trackingPaused",
                    "jobId",
                    "submissionId",
                    "requestId",
                ] {
                    obj.remove(key);
                }
            }
            result.insert(format!("generation:{id}"), value);
        }
    }
    for name in ["brief", "creation"] {
        result.insert(name.into(), document[name].clone());
    }
    for value in result.values_mut() {
        canonical(value);
    }
    result
}
pub fn read_targets(result: &Value) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for (name, prefix) in [
        ("nodes", "node"),
        ("relevantNodes", "node"),
        ("details", "node"),
        ("assets", "asset"),
        ("clips", "clip"),
        ("tracks", "track"),
    ] {
        collect(&mut keys, &result[name], prefix);
    }
    for id in result["missingNodeIds"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
    {
        keys.insert(format!("node:{id}"));
    }
    if let Some(section) = result["section"].as_str() {
        let prefix = match section {
            "assets" => "asset",
            "clips" => "clip",
            "tracks" => "track",
            "captions" => "caption",
            "generation" => "generation",
            _ => "",
        };
        collect(&mut keys, &result["items"], prefix);
    }
    if !result["requirements"].is_null()
        || !result["brief"].is_null()
        || result["section"] == "creation"
    {
        keys.insert("brief".into());
    }
    if result.get("creation").is_some() {
        keys.insert("creation".into());
    }
    keys
}
fn collect(keys: &mut BTreeSet<String>, rows: &Value, prefix: &str) {
    for item in rows.as_array().into_iter().flatten() {
        if let Some(id) = item["id"].as_str() {
            keys.insert(format!("{prefix}:{id}"));
        }
    }
}
pub fn remember(
    db: &Connection,
    project: &str,
    turn: &str,
    keys: &BTreeSet<String>,
    document: &Value,
) -> Result<()> {
    let values = targets(document);
    for key in keys {
        let value = values.get(key).unwrap_or(&Value::Null);
        db.execute("INSERT INTO project_observations(project_id,turn_id,target,value) VALUES(?1,?2,?3,?4) ON CONFLICT(project_id,turn_id,target) DO UPDATE SET value=excluded.value", params![project, turn, key, version(value)])?;
    }
    Ok(())
}
pub fn required(before: &Value, after: &Value, args: &Value) -> BTreeSet<String> {
    let old = targets(before);
    let new = targets(after);
    // Every modified existing object, including indirect timeline/shot changes,
    // participates in the same precondition check before the transaction writes.
    let mut keys: BTreeSet<_> = old
        .iter()
        .filter(|(key, value)| new.get(*key) != Some(*value))
        .map(|(key, _)| key.clone())
        .collect();
    for op in args["operations"].as_array().into_iter().flatten() {
        let prefix = match op["op"].as_str().unwrap_or("") {
            "update_node"
            | "remove_node"
            | "set_references"
            | "choose_take"
            | "assemble_screenplay"
            | "request_generation" => "node",
            "update_clip" | "move_clip" | "retime_clip" | "slip_clip" | "set_transition"
            | "remove_clip" => "clip",
            "update_track" => "track",
            "update_caption" | "remove_caption" => "caption",
            _ => "",
        };
        if !prefix.is_empty()
            && let Some(id) = op["id"].as_str()
        {
            keys.insert(format!("{prefix}:{id}"));
        }
        if let Some(id) = op["fromClipId"].as_str() {
            keys.insert(format!("clip:{id}"));
        }
        if let Some(id) = op["taskKey"].as_str() {
            keys.insert(format!("generation:{id}"));
        }
        if op["op"] == "assemble_screenplay" {
            for node in before["nodes"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|n| n["shot"]["screenplayId"] == op["id"])
            {
                keys.insert(format!("node:{}", node["id"].as_str().unwrap_or("")));
            }
        }
    }
    keys.retain(|key| old.contains_key(key));
    keys.extend(super::dependencies::required(before, args));
    keys
}
pub fn check(
    db: &Connection,
    project: &str,
    turn: &str,
    keys: &BTreeSet<String>,
    document: &Value,
) -> Result<Option<Value>> {
    let current = targets(document);
    let mut conflicts = vec![];
    for key in keys {
        let previous: Option<String> = db.query_row("SELECT value FROM project_observations WHERE project_id=?1 AND turn_id=?2 AND target=?3", params![project, turn, key], |r| r.get(0)).optional()?;
        let reason = match previous {
            None => "TARGET_NOT_OBSERVED",
            Some(raw) if version(current.get(key).unwrap_or(&Value::Null)) != raw => {
                "TARGET_CHANGED"
            }
            _ => continue,
        };
        let (section, id) = key.split_once(':').unwrap_or((key, ""));
        let inspect = match section {
            "node" => json!({"nodeIds":[id]}),
            "generation" => json!({"section":"generation","taskKey":id}),
            "brief" | "creation" => json!({"section":"creation"}),
            _ => json!({"section":format!("{section}s"),"ids":[id]}),
        };
        conflicts.push(json!({"target":key,"code":reason,"inspect":inspect}));
    }
    Ok((!conflicts.is_empty()).then(|| json!({"error":"Target observation required", "code":"TARGET_CONFLICT", "stage":"precondition", "outcome":"not_executed", "conflicts":conflicts, "recovery":{"action":"inspect_targets","message":"Read only the listed targets, then reconsider the edit. The whole batch was not executed."}})))
}
