//! Repair the temporary implementation that registered processing files as assets.
//! Retain all files and history; fold their IDs back into the original asset.
use super::*;
use std::collections::HashMap;

struct Row {
    project: String,
    owner: String,
    id: String,
    data: String,
    source: Option<String>,
    created: i64,
}
pub(super) fn run(db: &Connection) -> Result<()> {
    let columns = db
        .prepare("PRAGMA table_info(image_versions)")?
        .query_map([], |r| r.get::<_, String>(1))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if !columns.iter().any(|c| c == "asset_id") {
        return Ok(());
    }
    let rows=db.prepare("SELECT project_id,asset_id,id,data,provenance,created FROM image_versions ORDER BY created,rowid")?
        .query_map([],|r|Ok(Row {project:r.get(0)?,owner:r.get(1)?,id:r.get(2)?,data:r.get(3)?,source:r.get(4)?,created:r.get(5)?}))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let parents: HashMap<_, _> = rows
        .iter()
        .filter(|r| r.source.is_some())
        .map(|r| ((r.project.as_str(), r.id.as_str()), r.owner.as_str()))
        .collect();
    let mut groups: HashMap<(String, String), Vec<&Row>> = HashMap::new();
    for row in rows.iter().filter(|r| r.source.is_some()) {
        let mut root = row.owner.as_str();
        let mut visited = std::collections::HashSet::new();
        while let Some(parent) = parents.get(&(row.project.as_str(), root)) {
            ensure!(visited.insert(root), "图片版本来源循环");
            root = parent;
        }
        groups
            .entry((row.project.clone(), root.into()))
            .or_default()
            .push(row);
    }
    let tx = db.unchecked_transaction()?;
    tx.execute_batch("ALTER TABLE image_versions RENAME TO image_versions_legacy;")?;
    tx.execute_batch(SCHEMA)?;
    let mut documents: HashMap<String, Value> = HashMap::new();
    for ((project, root), versions) in groups {
        if !documents.contains_key(&project) {
            let raw: String = tx.query_row(
                "SELECT document FROM projects WHERE id=?1",
                [&project],
                |r| r.get(0),
            )?;
            documents.insert(project.clone(), serde_json::from_str(&raw)?);
        }
        let document = documents.get_mut(&project).unwrap();
        let raw: String =
            tx.query_row("SELECT data FROM assets WHERE id=?1", [&root], |r| r.get(0))?;
        let original: Value = serde_json::from_str(&raw)?;
        let mut asset = document["assets"]
            .as_array()
            .and_then(|a| a.iter().find(|a| a["id"] == root))
            .cloned()
            .unwrap_or_else(|| original.clone());
        let mut selected = asset["imageVersionId"]
            .as_str()
            .map(str::to_owned)
            .filter(|id| versions.iter().any(|v| v.id == *id));
        if selected.is_none() {
            selected = versions
                .iter()
                .find(|v| {
                    serde_json::from_str::<Value>(&v.data).is_ok_and(|f| f["path"] == asset["path"])
                })
                .map(|v| v.id.clone());
        }
        // The previous migration moved live canvas references to a derivative ID.
        if selected.is_none() {
            selected = versions
                .iter()
                .rev()
                .find(|v| {
                    references(&document["nodes"], &v.id) || references(&document["clips"], &v.id)
                })
                .map(|v| v.id.clone());
        }
        if selected.is_none() {
            selected = versions
                .iter()
                .rev()
                .find(|v| {
                    document["assets"].as_array().is_some_and(|a| {
                        a.iter().any(|a| a["id"] == v.id && a["inLibrary"] == true)
                    })
                })
                .map(|v| v.id.clone());
        }
        asset["imageId"] = json!(root);
        asset["imageVersionId"] = json!(selected.unwrap_or_else(|| original_id(&root)));
        for version in &versions {
            if document["assets"].as_array().is_some_and(|a| {
                a.iter()
                    .any(|a| a["id"] == version.id && a["inLibrary"] == true)
            }) {
                asset["inLibrary"] = json!(true);
            }
            remap(document, &version.id, &root);
            tx.execute(
                "DELETE FROM project_assets WHERE project_id=?1 AND asset_id=?2",
                params![project, version.id],
            )?;
        }
        let assets = document["assets"]
            .as_array_mut()
            .context("项目素材目录无效")?;
        assets.retain(|a| a["id"] != root && !versions.iter().any(|v| a["id"] == v.id));
        assets.push(asset);
        tx.execute(
            "INSERT INTO image_versions(project_id,image_id,id,data) VALUES(?1,?2,?3,?4)",
            params![project, root, original_id(&root), original.to_string()],
        )?;
        for row in versions {
            let mut file: Value = serde_json::from_str(&row.data)?;
            file["id"] = json!(row.id);
            tx.execute("INSERT INTO image_versions(project_id,image_id,id,data,provenance,created) VALUES(?1,?2,?3,?4,?5,?6)",params![project,root,row.id,file.to_string(),row.source,row.created])?;
        }
    }
    for (project, mut document) in documents {
        hydrate(&tx, &project, &mut document)?;
        document["storageVersion"] = json!(document["storageVersion"].as_u64().unwrap_or(0) + 1);
        tx.execute(
            "UPDATE projects SET document=?2 WHERE id=?1",
            params![project, document.to_string()],
        )?;
        crate::project_storage::remember(&tx, &project, &document)?;
    }
    tx.execute_batch("DROP TABLE image_versions_legacy;")?;
    // Only remove registry entries created by the mistaken collection path.
    // Image-version rows retain their files; shared/global material stays registered.
    for row in rows.iter().filter(|r| r.source.is_some()) {
        tx.execute("DELETE FROM assets WHERE id=?1 AND NOT EXISTS(SELECT 1 FROM project_assets WHERE asset_id=?1) AND NOT EXISTS(SELECT 1 FROM global_assets WHERE asset_id=?1)",[&row.id])?;
    }
    tx.commit()?;
    Ok(())
}
fn references(value: &Value, id: &str) -> bool {
    match value {
        Value::Object(fields) => fields.iter().any(|(key, v)| {
            (matches!(key.as_str(), "assetId" | "resultAssetId") && v == id) || references(v, id)
        }),
        Value::Array(items) => items.iter().any(|v| references(v, id)),
        _ => false,
    }
}
fn remap(value: &mut Value, from: &str, to: &str) {
    match value {
        Value::Object(fields) => {
            for (key, value) in fields {
                if matches!(
                    key.as_str(),
                    "assetId" | "resultAssetId" | "annotationSourceId"
                ) && *value == from
                {
                    *value = json!(to);
                } else if key == "resultAssetIds" {
                    if let Some(ids) = value.as_array_mut() {
                        for id in ids {
                            if *id == from {
                                *id = json!(to);
                            }
                        }
                    }
                } else {
                    remap(value, from, to);
                }
            }
        }
        Value::Array(items) => {
            for value in items {
                remap(value, from, to);
            }
        }
        _ => {}
    }
}
