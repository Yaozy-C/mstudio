use super::{cleanup::managed, references};
use anyhow::Result;
use rusqlite::Connection;
use serde_json::Value;
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
};

pub struct Plan {
    pub assets: HashSet<String>,
    pub files: HashSet<PathBuf>,
}

fn asset_paths(value: &Value, paths: &mut HashMap<String, HashSet<PathBuf>>) {
    match value {
        Value::Object(object) => {
            if let Some(id) = object.get("id").and_then(Value::as_str)
                && object.contains_key("path")
            {
                for key in ["path", "preview"] {
                    if let Some(path) = object
                        .get(key)
                        .and_then(Value::as_str)
                        .filter(|s| !s.is_empty())
                    {
                        paths
                            .entry(id.into())
                            .or_default()
                            .insert(PathBuf::from(path));
                    }
                }
            }
            for child in object.values() {
                asset_paths(child, paths);
            }
        }
        Value::Array(array) => {
            for child in array {
                asset_paths(child, paths);
            }
        }
        _ => {}
    }
}

pub fn build(db: &Connection, root: &Path, id: &str) -> Result<Plan> {
    let mut target = HashSet::new();
    let mut shared = HashSet::new();
    let mut global = db.prepare("SELECT asset_id FROM global_assets")?;
    for row in global.query_map([], |r| r.get::<_, String>(0))? {
        shared.insert(row?);
    }
    let mut paths = HashMap::new();
    let mut job_files = Vec::new();
    // Include stored conversation references and received results, not just current canvas nodes.
    for sql in [
        "SELECT id,document FROM projects",
        "SELECT project_id,data FROM jobs",
        "SELECT project_id,payload FROM agent_messages",
        "SELECT project_id,payload FROM agent_events",
    ] {
        let mut stmt = db.prepare(sql)?;
        let rows = stmt
            .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (owner, raw) in rows {
            let value: Value = serde_json::from_str(&raw)?;
            if value["providerId"] == "codex-image"
                && let (Some(base), Some(request)) = (
                    value["providerConfig"]["codexRoot"].as_str(),
                    value["requestId"].as_str(),
                )
                && !request.is_empty()
                && request
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || c == b'-')
                && let Some(path) = managed(root, &Path::new(base).join(request))?
            {
                job_files.push((owner.clone(), path));
            }
            references(
                &value,
                if owner == id {
                    &mut target
                } else {
                    &mut shared
                },
            );
            asset_paths(&value, &mut paths);
        }
    }
    let mut stmt = db.prepare("SELECT project_id,asset_id FROM project_assets")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (owner, asset) = row?;
        if owner == id {
            target.insert(asset);
        } else {
            shared.insert(asset);
        }
    }
    let mut stmt = db.prepare("SELECT data FROM assets")?;
    for row in stmt.query_map([], |r| r.get::<_, String>(0))? {
        asset_paths(&serde_json::from_str::<Value>(&row?)?, &mut paths);
    }
    target.retain(|asset| !shared.contains(asset));
    let mut candidates = HashSet::new();
    let mut protected = HashSet::new();
    for (owner, path) in job_files {
        if owner == id {
            candidates.insert(path);
        } else {
            protected.insert(path);
        }
    }
    for (asset, files) in paths {
        for path in files {
            if let Some(path) = managed(root, &path)? {
                if target.contains(&asset) {
                    candidates.insert(path);
                } else {
                    protected.insert(path);
                }
            }
        }
    }
    let mut stmt = db.prepare("SELECT project_id,path FROM project_files")?;
    for row in stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))? {
        let (owner, path) = row?;
        if let Some(path) = managed(root, Path::new(&path))? {
            if owner == id {
                candidates.insert(path);
            } else {
                protected.insert(path);
            }
        }
    }
    // Video proxy cache names contain the exact source asset ID plus a fixed delimiter.
    let proxies = root.join("proxies");
    if proxies.is_dir() {
        for entry in std::fs::read_dir(proxies)? {
            let entry = entry?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if target
                .iter()
                .any(|asset| name.starts_with(&format!("{asset}-v1-")))
                && let Some(path) = managed(root, &entry.path())?
            {
                candidates.insert(path);
            }
        }
    }
    candidates.retain(|path| {
        !protected
            .iter()
            .any(|keep| path.starts_with(keep) || keep.starts_with(path))
    });
    Ok(Plan {
        assets: target,
        files: candidates,
    })
}
