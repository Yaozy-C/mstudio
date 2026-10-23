//! A generation task owns the display name of its outputs across persisted views.
use anyhow::{Context, Result};
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::collections::HashMap;

pub fn result_name(db: &Connection, job: &Value) -> Result<String> {
    let project = job["projectId"].as_str().context("任务缺少项目")?;
    let document = super::load(db, project)?;
    let value =
        super::runtime::execute(json!({"action":"result_name","document":document,"job":job}))?;
    if let Some(error) = value.get("error") {
        anyhow::bail!("生成结果命名失败：{error}");
    }
    value["name"]
        .as_str()
        .map(str::to_owned)
        .context("生成结果缺少任务名称")
}

/// Caller owns the transaction. Only output names owned by this project propagate.
pub fn sync(db: &Connection, before: &Value, after: &Value) -> Result<()> {
    let Some(project) = after["id"].as_str() else {
        return Ok(());
    };
    let old: HashMap<_, _> = before["assets"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|asset| Some((asset["id"].as_str()?, asset["name"].as_str()?)))
        .collect();
    for asset in after["assets"].as_array().into_iter().flatten() {
        let (Some(id), Some(name)) = (asset["id"].as_str(), asset["name"].as_str()) else {
            continue;
        };
        if old.get(id) == Some(&name) {
            continue;
        }
        let jobs = db.prepare("SELECT id,data FROM jobs WHERE project_id=?1 AND (json_extract(data,'$.asset.id')=?2 OR EXISTS(SELECT 1 FROM json_each(data,'$.assets') WHERE json_extract(value,'$.id')=?2))")?
            .query_map(params![project, id], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        if jobs.is_empty() {
            continue;
        }
        db.execute(
            "UPDATE assets SET data=json_set(data,'$.name',?2) WHERE id=?1",
            params![id, name],
        )?;
        for (job_id, raw) in jobs {
            let mut job: Value = serde_json::from_str(&raw)?;
            if job["asset"]["id"] == id {
                job["asset"]["name"] = json!(name);
            }
            for output in job["assets"].as_array_mut().into_iter().flatten() {
                if output["id"] == id {
                    output["name"] = json!(name);
                }
            }
            // The task snapshot and blob references are immutable request facts.
            db.execute(
                "UPDATE jobs SET data=?2 WHERE id=?1",
                params![job_id, job.to_string()],
            )?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn renames_owned_output_metadata_and_preserves_files_and_foreign_assets() {
        let (store, mut doc, _) = super::super::tests::fixture();
        let db = store.conn().unwrap();
        let asset = json!({"id":"output","name":"Old","path":"/synthetic-source.png"});
        db.execute(
            "INSERT INTO assets VALUES('output',?1)",
            [asset.to_string()],
        )
        .unwrap();
        let job =
            json!({"id":"j","asset":asset,"assets":[asset],"input":{"prompt":"original prompt"}});
        db.execute("INSERT INTO jobs VALUES('j','p',?1)", [job.to_string()])
            .unwrap();
        doc["assets"] = json!([asset]);
        let mut renamed = doc.clone();
        renamed["assets"][0]["name"] = json!("小满 · 人物资产");
        sync(&db, &doc, &renamed).unwrap();
        let raw: String = db
            .query_row("SELECT data FROM assets WHERE id='output'", [], |r| {
                r.get(0)
            })
            .unwrap();
        let renamed_asset: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(renamed_asset["name"], "小满 · 人物资产");
        assert_eq!(renamed_asset["path"], asset["path"]);
        let raw: String = db
            .query_row("SELECT data FROM jobs WHERE id='j'", [], |r| r.get(0))
            .unwrap();
        let renamed_job: Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(renamed_job["asset"]["name"], renamed_asset["name"]);
        assert_eq!(renamed_job["assets"][0]["name"], renamed_asset["name"]);
        assert_eq!(renamed_job["input"], job["input"]);
        renamed["id"] = json!("another-project");
        renamed["assets"][0]["name"] = json!("Foreign rename");
        sync(&db, &Value::Null, &renamed).unwrap();
        let raw: String = db
            .query_row("SELECT data FROM assets WHERE id='output'", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(serde_json::from_str::<Value>(&raw).unwrap(), renamed_asset);
        drop(db);
        let root = store.root.clone();
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
