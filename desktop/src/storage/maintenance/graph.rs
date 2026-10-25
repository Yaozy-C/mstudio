use anyhow::Result;
use rusqlite::Connection;
use serde_json::Value;
use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

pub struct Graph {
    pub ids: HashSet<String>,
    pub paths: HashSet<PathBuf>,
    pub assets: Vec<Value>,
    pub private: Vec<(String, Value)>,
    pub documents: Vec<(String, Value)>,
}
fn strings(value: &Value, out: &mut Vec<String>) {
    match value {
        Value::String(s) => out.push(s.clone()),
        Value::Array(a) => a.iter().for_each(|v| strings(v, out)),
        Value::Object(o) => o.values().for_each(|v| strings(v, out)),
        _ => (),
    }
}
fn library(a: &Value) -> bool {
    a["annotationSourceId"]
        .as_str()
        .is_none_or(|s| s.is_empty())
        && a["inLibrary"]
            .as_bool()
            .unwrap_or(a["generated"] != true && a["attachmentOnly"] != true)
}
fn json_rows(db: &Connection, sql: &str) -> Result<Vec<Value>> {
    db.prepare(sql)?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?
        .iter()
        .map(|s| serde_json::from_str(s).map_err(Into::into))
        .collect()
}
pub fn build(db: &Connection, root: &Path) -> Result<Graph> {
    let assets = json_rows(db, "SELECT data FROM assets")?;
    let mut graph = Graph {
        ids: HashSet::new(),
        paths: HashSet::new(),
        assets,
        private: vec![],
        documents: vec![],
    };
    let mut roots = vec![];
    for raw in json_rows(
        db,
        "SELECT json_object('id',id,'doc',json(document)) FROM projects",
    )? {
        let id = raw["id"].as_str().unwrap().to_owned();
        let document = raw["doc"].clone();
        for asset in document["assets"]
            .as_array()
            .into_iter()
            .flatten()
            .filter(|a| library(a))
        {
            if let Some(id) = asset["id"].as_str() {
                graph.ids.insert(id.into());
            }
        }
        let mut live = document.clone();
        for key in ["assets", "previsAssets", "removedAssetIds"] {
            live.as_object_mut().unwrap().remove(key);
        }
        roots.push(live);
        graph.documents.push((id, document));
    }
    for sql in [
        "SELECT data FROM jobs",
        "SELECT json_object('content',content,'payload',json(payload)) FROM agent_messages",
        "SELECT json_object('output',output) FROM subagent_runs",
        "SELECT json_object('text',text,'source',source) FROM subagent_inbox",
        "SELECT json_object('output',output) FROM subagent_notices",
        "SELECT json_object('text',text) FROM previs_messages",
        "SELECT data FROM previs_exports",
        "SELECT data FROM previs_files",
        // Versions remain user-owned even while not selected as the reference.
        "SELECT data FROM image_versions",
    ] {
        roots.extend(json_rows(db, sql)?);
    }
    pinned_receipts(db, &mut roots)?;
    for id in db
        .prepare("SELECT asset_id FROM global_assets")?
        .query_map([], |r| r.get::<_, String>(0))?
    {
        graph.ids.insert(id?);
    }
    for raw in json_rows(
        db,
        "SELECT json_object('project',project_id,'asset',json(data)) FROM previs_media",
    )? {
        graph.private.push((
            raw["project"].as_str().unwrap().into(),
            raw["asset"].clone(),
        ));
    }
    let metadata: Vec<_> = graph
        .assets
        .iter()
        .chain(graph.private.iter().map(|(_, a)| a))
        .chain(
            graph
                .documents
                .iter()
                .flat_map(|(_, d)| d["assets"].as_array().into_iter().flatten()),
        )
        .cloned()
        .collect();
    // A completed import may not yet have reached the UI's saved document.
    for asset in &metadata {
        let fresh = asset["path"]
            .as_str()
            .and_then(|p| std::fs::metadata(p).ok())
            .and_then(|m| m.modified().ok())
            .is_some_and(|t| t.elapsed().unwrap_or_default() < std::time::Duration::from_secs(300));
        if fresh && let Some(id) = asset["id"].as_str() {
            graph.ids.insert(id.into());
        }
    }
    let candidates: Vec<_> = metadata
        .iter()
        .filter_map(|a| a["id"].as_str().map(str::to_owned))
        .collect();
    let mut text = vec![];
    for value in &roots {
        crate::project_storage::references(value, &mut graph.ids);
        strings(value, &mut text);
    }
    // Text may contain executable source paths or explicitly named media IDs.
    for value in &text {
        for id in &candidates {
            if value.contains(id) {
                graph.ids.insert(id.clone());
            }
        }
        if value.starts_with(root.to_string_lossy().as_ref()) {
            graph.paths.insert(PathBuf::from(value));
        }
    }
    // Follow dependencies such as annotated images back to their source originals.
    loop {
        let before = graph.ids.len();
        for asset in &metadata {
            if asset["id"]
                .as_str()
                .is_some_and(|id| graph.ids.contains(id))
            {
                if let Some(id) = asset["annotationSourceId"].as_str() {
                    graph.ids.insert(id.into());
                }
                for key in ["path", "preview"] {
                    if let Some(path) = asset[key].as_str().filter(|s| !s.is_empty()) {
                        graph.paths.insert(path.into());
                    }
                }
            }
        }
        if before == graph.ids.len() {
            break;
        }
    }
    for row in db
        .prepare("SELECT project_id,data FROM previs_files")?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
    {
        let (project, raw) = row?;
        let file: crate::previs::File = serde_json::from_str(&raw)?;
        graph
            .paths
            .insert(root.join("previs").join(project).join(file.id).join(
                if file.kind == "model" {
                    "model.js"
                } else {
                    "scene.js"
                },
            ));
    }
    for digest in db.prepare("SELECT DISTINCT media_digest FROM content_blobs b WHERE media_digest IS NOT NULL AND (EXISTS(SELECT 1 FROM job_content WHERE digest=b.digest) OR EXISTS(SELECT 1 FROM event_content WHERE digest=b.digest))")?
        .query_map([], |r| r.get::<_, String>(0))? {
        graph.paths.insert(root.join("reference-media").join(digest?));
    }
    // Source paths can occur inside JavaScript, including paths outside the asset table.
    let mut source = vec![];
    for value in &roots {
        strings(value, &mut source);
    }
    for value in source {
        for fragment in value.split(['"', '\'', '`', '\n', '\r']) {
            if fragment.starts_with(root.to_string_lossy().as_ref()) {
                graph.paths.insert(fragment.into());
            }
        }
    }
    let normalized: Vec<_> = graph
        .paths
        .iter()
        .filter_map(|p| p.canonicalize().ok())
        .collect();
    graph.paths.extend(normalized);
    Ok(graph)
}

fn pinned_receipts(db: &Connection, roots: &mut Vec<Value>) -> Result<()> {
    let mut events = db
        .prepare("SELECT seq,payload FROM agent_events WHERE kind='tool/result'")?
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    loop {
        let mut text = vec![];
        for root in roots.iter() {
            strings(root, &mut text);
        }
        let mut next = vec![];
        let mut added = false;
        for (seq, raw) in events {
            let value: Value = serde_json::from_str(&raw)?;
            if value["callId"]
                .as_str()
                .is_some_and(|id| !id.is_empty() && text.iter().any(|s| s.contains(id)))
            {
                roots.push(crate::database::blobs::text_event(db, seq, &raw)?);
                added = true;
            } else {
                next.push((seq, raw));
            }
        }
        if !added {
            break;
        }
        events = next;
    }
    Ok(())
}
