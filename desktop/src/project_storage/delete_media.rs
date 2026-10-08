//! Permanent project media deletion. Keep only ID tombstones for completed
//! outputs, so workers neither download them again nor resurrect canvas cards.
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::{collections::HashSet, path::Path};

fn ids(document: &Value) -> HashSet<String> {
    document["removedAssetIds"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect()
}
fn deleted(value: &Value, removed: &HashSet<String>) -> bool {
    value["id"].as_str().is_some_and(|id| removed.contains(id))
}
// Works on packed jobs as well as in-flight, hydrated worker snapshots.
pub fn redact_job(job: &mut Value, removed: &HashSet<String>) -> Vec<String> {
    let mut prefixes = Vec::new();
    if removed.is_empty() {
        return prefixes;
    }
    let mut outputs = Vec::new();
    for (i, asset) in job["assets"].as_array().into_iter().flatten().enumerate() {
        if deleted(asset, removed) {
            outputs.push(i);
        }
    }
    if deleted(&job["asset"], removed) && !outputs.contains(&0) {
        outputs.push(0);
    }
    if !outputs.is_empty() {
        prefixes.push("/result/".into());
        job["result"] = Value::Null;
    }
    for i in outputs {
        prefixes.push(format!("/outputs/{i}/"));
        if let Some(output) = job["outputs"].get_mut(i) {
            *output = json!({"deleted":true});
        }
        if let Some(asset) = job["assets"].get_mut(i) {
            *asset = json!({"id":asset["id"],"deleted":true});
        }
    }
    if deleted(&job["asset"], removed) {
        job["asset"] = json!({"id":job["asset"]["id"],"deleted":true});
    }
    let inputs = &job["shot"]["canvasGeneration"]["task"]["inputs"];
    let affected = inputs.as_array().is_some_and(|refs| {
        refs.iter()
            .any(|r| r["assetId"].as_str().is_some_and(|id| removed.contains(id)))
    });
    if affected {
        // Provider request formats differ. Retain the prompt, drop its media
        // payload instead of retaining a hidden copy of a deleted reference.
        let prompt = job["shot"]["canvasGeneration"]["task"]["prompt"].clone();
        job["input"] = json!({"prompt":prompt,"referencesDeleted":true});
        prefixes.push("/input/".into());
    }
    prefixes
}

pub fn sanitize_job(db: &Connection, job: &mut Value) -> Result<()> {
    let Some(project) = job["projectId"].as_str() else {
        return Ok(());
    };
    let document = crate::project_service::load(db, project)?;
    redact_job(job, &ids(&document));
    Ok(())
}

pub fn commit(db: &Connection, before: &Value, after: &Value) -> Result<()> {
    let removed = ids(after);
    let added: HashSet<_> = removed.difference(&ids(before)).cloned().collect();
    if added.is_empty() {
        return Ok(());
    }
    let project = after["id"].as_str().context("Missing project ID")?;
    let database: String = db.query_row(
        "SELECT file FROM pragma_database_list WHERE name='main'",
        [],
        |r| r.get(0),
    )?;
    let fallback = Path::new(&database)
        .parent()
        .context("Missing database directory")?;
    let root = crate::storage::load(db, fallback)?.directory;
    let plan = super::plan::build_selected(db, &root, project, Some(&added))?;
    for path in plan.files {
        db.execute(
            "INSERT OR IGNORE INTO pending_file_deletions VALUES(?1,?2)",
            params![project, path.to_string_lossy()],
        )?;
        db.execute(
            "DELETE FROM project_files WHERE project_id=?1 AND path=?2",
            params![project, path.to_string_lossy()],
        )?;
    }
    for id in &added {
        db.execute(
            "DELETE FROM project_assets WHERE project_id=?1 AND asset_id=?2",
            params![project, id],
        )?;
    }
    for id in plan.assets {
        db.execute("DELETE FROM assets WHERE id=?1", [id])?;
    }
    let jobs = db
        .prepare("SELECT id,data FROM jobs WHERE project_id=?1")?
        .query_map([project], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let mut digests = HashSet::new();
    for (id, raw) in jobs {
        let mut job: Value = serde_json::from_str(&raw)?;
        let prefixes = redact_job(&mut job, &removed);
        if prefixes.is_empty() {
            continue;
        }
        let refs = db
            .prepare("SELECT pointer,digest FROM job_content WHERE owner=?1")?
            .query_map([&id], |r| {
                Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (pointer, digest) in refs {
            if prefixes
                .iter()
                .any(|prefix| pointer == *prefix || pointer.starts_with(prefix))
            {
                digests.insert(digest);
                db.execute(
                    "DELETE FROM job_content WHERE owner=?1 AND pointer=?2",
                    params![id, pointer],
                )?;
            }
        }
        db.execute(
            "UPDATE jobs SET data=?1 WHERE id=?2",
            params![job.to_string(), id],
        )?;
    }
    // The same generated bytes can also occur in chat/tool responses. Detach
    // this project's copies, but leave other projects' snapshots intact.
    let paths: HashSet<String> = before["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter(|a| deleted(a, &added))
        .flat_map(|a| {
            ["path", "preview"]
                .into_iter()
                .filter_map(move |key| a[key].as_str().map(str::to_owned))
        })
        .collect();
    super::deleted_media_events::retire_files(db, project, &paths)?;
    super::deleted_media_events::retire(db, project, &digests)?;
    crate::database::blobs::collect(db)?;
    Ok(())
}
