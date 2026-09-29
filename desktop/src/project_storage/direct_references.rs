//! Retire the short-lived subject/view registry. Only compatibility migration
//! knows its fields; active projects and tools use ordinary image references.
use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
};

type Views = HashMap<(String, String), (Option<String>, String)>;
fn views(document: &Value) -> Views {
    let mut result = Views::new();
    for record in document["sharedAssets"].as_array().into_iter().flatten() {
        for view in record["views"].as_array().into_iter().flatten() {
            if let (Some(subject), Some(id)) = (record["id"].as_str(), view["id"].as_str()) {
                result.insert(
                    (subject.into(), id.into()),
                    (
                        view["assetId"].as_str().map(str::to_owned),
                        format!(
                            "{} · {}",
                            record["name"].as_str().unwrap_or(subject),
                            view["name"].as_str().unwrap_or(id)
                        ),
                    ),
                );
            }
        }
    }
    result
}
fn flatten(value: &mut Value, index: &Views) {
    match value {
        Value::Object(object) => {
            if let Some(Value::Array(old)) = object.remove("sharedReferences") {
                let mut references = object
                    .get("references")
                    .and_then(Value::as_array)
                    .cloned()
                    .unwrap_or_default();
                let mut missing = Vec::new();
                for reference in old {
                    let key = (
                        reference["sharedAssetId"].as_str().unwrap_or("").to_owned(),
                        reference["viewId"].as_str().unwrap_or("").to_owned(),
                    );
                    match index.get(&key) {
                        Some((Some(id), label)) => {
                            if !references.iter().any(|r| r["assetId"] == *id) {
                                references.push(json!({"assetId":id,"purpose":reference["purpose"].as_str().unwrap_or(label)}));
                            }
                        }
                        view => missing.push(
                            view.map(|(_, label)| label.clone())
                                .unwrap_or_else(|| format!("{} · {}", key.0, key.1)),
                        ),
                    }
                }
                object.insert("references".into(), json!(references));
                // A planned view with no image cannot become a valid reference.
                // Keep its intent as a plain note instead of inventing an asset ID.
                if !missing.is_empty() {
                    let text = object.get("text").and_then(Value::as_str).unwrap_or("");
                    object.insert(
                        "text".into(),
                        json!(format!("{text}\n参考图片待补充：{}", missing.join("；"))),
                    );
                }
            }
            object.remove("sharedAssets");
            object.remove("assetTarget");
            for child in object.values_mut() {
                flatten(child, index);
            }
        }
        Value::Array(array) => {
            for child in array {
                flatten(child, index);
            }
        }
        _ => {}
    }
}
pub fn document(document: &mut Value) {
    convert(document, false);
}
fn convert(document: &mut Value, collect_results: bool) {
    let index = views(document);
    let mut collected: HashSet<String> = index.values().filter_map(|(id, _)| id.clone()).collect();
    if let Some(drafts) = document["production"]["drafts"].as_object() {
        for task in drafts
            .values()
            .filter(|t| collect_results && t["generationPurpose"] == "asset")
        {
            if let Some(id) = task["resultAssetId"].as_str() {
                collected.insert(id.into());
            }
            for id in task["resultAssetIds"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(Value::as_str)
            {
                collected.insert(id.into());
            }
        }
    }
    if let Some(assets) = document["assets"].as_array_mut() {
        for asset in assets {
            if asset["id"]
                .as_str()
                .is_some_and(|id| collected.contains(id))
            {
                asset["inLibrary"] = json!(true);
            }
        }
    }
    if let Some(removed) = document["removedAssetIds"].as_array_mut() {
        removed.retain(|id| !id.as_str().is_some_and(|id| collected.contains(id)));
    }
    flatten(document, &index);
}
fn rewrite(text: &str, replacements: &[(String, String)]) -> String {
    replacements
        .iter()
        .fold(text.to_owned(), |s, (old, new)| s.replace(old, new))
}
pub fn migrate(db: &Connection, root: &Path) -> Result<()> {
    const KEY: &str = "direct_image_references_v1";
    if db.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key=?1)",
        [KEY],
        |r| r.get::<_, bool>(0),
    )? {
        return Ok(());
    }
    // SQLite creates a consistent full backup, including WAL contents, before
    // the transaction updates project documents and saved instructions.
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_nanos();
    let backup = root.join(format!("mstudio-before-direct-references-{stamp}.sqlite3"));
    db.execute("VACUUM INTO ?1", [backup.to_string_lossy().as_ref()])?;
    let tx = db.unchecked_transaction()?;
    let projects = tx
        .prepare("SELECT id,document FROM projects")?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut indices = HashMap::new();
    for (id, raw) in projects {
        let mut value: Value = serde_json::from_str(&raw)?;
        indices.insert(id.clone(), views(&value));
        let old = value.clone();
        convert(&mut value, true);
        if value != old {
            super::remember(&tx, &id, &old)?;
            tx.execute(
                "UPDATE projects SET document=?2 WHERE id=?1",
                params![id, value.to_string()],
            )?;
        }
    }
    let jobs = tx
        .prepare("SELECT id,project_id,data FROM jobs")?
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (id, project, raw) in jobs {
        let mut value: Value = serde_json::from_str(&raw)?;
        let old = value.clone();
        flatten(&mut value, indices.get(&project).unwrap_or(&Views::new()));
        if value != old {
            tx.execute(
                "UPDATE jobs SET data=?2 WHERE id=?1",
                params![id, value.to_string()],
            )?;
        }
    }
    let replacements: Vec<(String, String)> =
        serde_json::from_str(include_str!("direct_reference_rules.json"))?;
    let settings = tx
        .prepare("SELECT value FROM settings WHERE key='agents'")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for raw in settings {
        let mut agents: Value = serde_json::from_str(&raw)?;
        for agent in agents.as_array_mut().into_iter().flatten() {
            let old = agent.clone();
            for field in ["instructions", "description"] {
                if let Some(text) = agent[field].as_str() {
                    agent[field] = json!(rewrite(text, &replacements));
                }
            }
            if *agent != old {
                agent["revision"] = json!(agent["revision"].as_u64().unwrap_or(0) + 1);
            }
        }
        tx.execute(
            "UPDATE settings SET value=?1 WHERE key='agents'",
            [agents.to_string()],
        )?;
    }
    let has_rules: bool = tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='skill_resources')",
        [],
        |r| r.get(0),
    )?;
    if has_rules {
        let rules = tx
            .prepare("SELECT skill_id,path,text FROM skill_resources")?
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, String>(2)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (id, path, old) in rules {
            let text = rewrite(&old, &replacements);
            if text != old {
                tx.execute("UPDATE skill_resources SET text=?3,revision=revision+1,updated=unixepoch() WHERE skill_id=?1 AND path=?2", params![id, path, text])?;
            }
        }
    }
    tx.execute(
        "INSERT INTO settings(key,value) VALUES(?1,?2)",
        params![KEY, backup.to_string_lossy()],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "direct_references_tests.rs"]
mod tests;
