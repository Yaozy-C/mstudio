//! The library reads card metadata; full documents are loaded only when opened.
use super::Store;
use anyhow::Result;
use serde_json::{Value, json};
use std::path::Path;

impl Store {
    pub fn project_summaries(&self) -> Result<Vec<Value>> {
        let started = std::time::Instant::now();
        let rows = {
            let db = self.conn()?;
            let mut stmt = db.prepare(
                "SELECT id,name,updated,json_extract(document,'$.width'),
                json_extract(document,'$.height'),json_array_length(document,'$.clips'),
                (SELECT json_group_array(json_object('path',json_extract(a.value,'$.path'),
                    'preview',json_extract(a.value,'$.preview')))
                 FROM json_each(document,'$.assets') a
                 WHERE coalesce(json_extract(a.value,'$.preview'),'')!='')
                FROM projects ORDER BY updated DESC",
            )?;
            stmt.query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, i64>(3)?,
                    r.get::<_, i64>(4)?,
                    r.get::<_, i64>(5)?,
                    r.get::<_, String>(6)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
        };
        let summaries = rows
            .into_iter()
            .map(|(id, name, updated, width, height, clip_count, covers)| {
                let covers: Vec<Value> = serde_json::from_str(&covers)?;
                let preview = covers
                    .iter()
                    .find(|a| a["path"].as_str().is_some_and(|p| Path::new(p).is_file()))
                    .and_then(|a| a["preview"].as_str())
                    .unwrap_or("");
                Ok(json!({"id":id,"name":name,"updated":updated,"width":width,
                "height":height,"clipCount":clip_count,"preview":preview}))
            })
            .collect::<Result<Vec<_>>>()?;
        tracing::info!(
            event = "project_library_loaded",
            duration_ms = started.elapsed().as_millis() as u64,
            project_count = summaries.len()
        );
        Ok(summaries)
    }

    pub fn project(&self, id: &str) -> Result<Value> {
        let (mut document, generated) = {
            let db = self.conn()?;
            (
                crate::project_service::load(&db, id)?,
                crate::asset_library::generated_ids(&db)?,
            )
        };
        if let Some(assets) = document["assets"].as_array_mut() {
            for asset in assets {
                asset["missing"] = json!(
                    !asset["path"]
                        .as_str()
                        .is_some_and(|p| Path::new(p).is_file())
                );
                if asset["id"]
                    .as_str()
                    .is_some_and(|id| generated.contains(id))
                {
                    asset["generated"] = json!(true);
                }
            }
        }
        Ok(document)
    }
}
