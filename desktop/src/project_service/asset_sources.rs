//! Asset provenance comes from committed job outputs, never filenames or chat text.
use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

pub fn enrich(db: &Connection, project: &str, args: &Value, result: &mut Value) -> Result<()> {
    if args["section"] != "assets"
        || args["fields"]
            .as_array()
            .is_some_and(|f| !f.iter().any(|v| v == "source"))
    {
        return Ok(());
    }
    let detailed = args["ids"].is_array() || args["fields"].is_array();
    let offset = args["textOffset"].as_u64().unwrap_or(0) as usize;
    let limit = if args["ids"].as_array().is_some_and(|ids| ids.len() == 1) {
        4000
    } else {
        1000
    };
    let Some(items) = result["items"].as_array_mut() else {
        return Ok(());
    };
    let mut size = 0;
    let mut kept = 0;
    for item in items.iter_mut() {
        let id = item["id"].as_str().unwrap_or("");
        item["source"] = source(db, project, id, detailed, offset, limit)?;
        let bytes = item.to_string().len();
        if kept > 0 && size + bytes > 12000 {
            break;
        }
        size += bytes;
        kept += 1;
    }
    items.truncate(kept);
    let next = args["offset"].as_u64().unwrap_or(0) + kept as u64;
    result["nextOffset"] = if next < result["total"].as_u64().unwrap_or(0) {
        json!(next)
    } else {
        Value::Null
    };
    Ok(())
}
fn source(
    db: &Connection,
    project: &str,
    asset: &str,
    detailed: bool,
    offset: usize,
    limit: usize,
) -> Result<Value> {
    let mut stmt = db.prepare("SELECT id,data FROM jobs WHERE project_id=?1 AND (json_extract(data,'$.asset.id')=?2 OR EXISTS(SELECT 1 FROM json_each(data,'$.assets') a WHERE json_extract(a.value,'$.id')=?2)) ORDER BY id")?;
    let jobs = stmt
        .query_map(params![project, asset], |r| {
            Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    if jobs.is_empty() {
        return Ok(
            json!({"status":"unavailable","reason":"No generating job is linked to this asset in this project."}),
        );
    }
    if jobs.len() != 1 {
        return Ok(
            json!({"status":"ambiguous","jobIds":jobs.iter().map(|j| &j.0).collect::<Vec<_>>() }),
        );
    }
    let (id, raw) = &jobs[0];
    let job: Value = serde_json::from_str(raw)?;
    let mut result = json!({"status":"available","jobId":id,"taskKey":job["shot"]["canvasGeneration"]["task"]["key"],"modelId":job["mediaModelId"],"providerId":job["providerId"]});
    if detailed {
        // Hydrate the submitted text only. Binary inputs, credentials and provider
        // configuration must never enter the model's project inspection result.
        let text = crate::database::blobs::hydrate(
            db,
            crate::database::blobs::Owner::Job(id),
            json!({"input":{"prompt":job["input"]["prompt"]}}),
        )?;
        result["prompt"] = match text["input"]["prompt"].as_str() {
            Some(text) => {
                let total = text.chars().count();
                json!({"text":text.chars().skip(offset).take(limit).collect::<String>(),"nextTextOffset":if offset.saturating_add(limit)<total {Some(offset+limit)} else {None}})
            }
            None => Value::Null,
        };
        result["parameters"] = json!(
            [
                "model",
                "duration",
                "resolution",
                "aspect_ratio",
                "aspectRatio",
                "size",
                "quality",
                "n",
                "seed",
                "fps"
            ]
            .iter()
            .filter_map(|key| job["input"]
                .get(*key)
                .filter(|v| v.is_number()
                    || v.is_boolean()
                    || v.as_str().is_some_and(|s| s.len() <= 200))
                .map(|v| ((*key).to_owned(), v.clone())))
            .collect::<serde_json::Map<_, _>>()
        );
        result["references"] = json!(
            job["shot"]["references"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|r| json!({"assetId":r["assetId"],"purpose":r["purpose"],"role":r["role"]}))
                .collect::<Vec<_>>()
        );
    }
    Ok(result)
}

#[cfg(test)]
#[path = "asset_sources_tests.rs"]
mod tests;
