//! Display history is durable as soon as a request is accepted; pixels stay out of SQLite.
use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::{Value, json};

pub fn init(db: &Connection) -> Result<()> {
    for (column, definition) in [
        ("images", "TEXT NOT NULL DEFAULT '[]'"),
        ("request_id", "TEXT"),
    ] {
        let exists: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM pragma_table_info('previs_messages') WHERE name=?1)",
            [column],
            |r| r.get(0),
        )?;
        if !exists {
            db.execute_batch(&format!(
                "ALTER TABLE previs_messages ADD COLUMN {column} {definition}"
            ))?;
        }
    }
    Ok(())
}

pub fn images(store: &crate::database::Store, doc: &Value, ids: &[String]) -> Result<Value> {
    let assets = super::media::resolved(store, doc)?;
    let mut seen = std::collections::HashSet::new();
    Ok(json!(
        ids.iter()
            .filter(|id| seen.insert(*id))
            .filter_map(|id| {
                assets
                    .iter()
                    .find(|a| a.id == *id)
                    .map(|a| json!({"id":a.id,"name":a.name,"path":a.path,"preview":a.preview}))
            })
            .collect::<Vec<_>>()
    ))
}

pub fn save(
    db: &Connection,
    project: &str,
    file: &str,
    request: &str,
    role: &str,
    text: &str,
    images: &Value,
) -> Result<()> {
    db.execute(
        "INSERT INTO previs_messages(project_id,file_id,request_id,role,text,images) VALUES(?1,?2,?3,?4,?5,?6)",
        params![project, file, request, role, text, images.to_string()],
    )?;
    db.execute("DELETE FROM previs_messages WHERE project_id=?1 AND file_id=?2 AND id NOT IN (SELECT id FROM previs_messages WHERE project_id=?1 AND file_id=?2 ORDER BY id DESC LIMIT 100)", params![project, file])?;
    Ok(())
}

pub fn list(db: &Connection, project: &str) -> Result<Vec<Value>> {
    let rows = db.prepare("SELECT id,file_id,role,text,images,request_id FROM previs_messages WHERE project_id=?1 ORDER BY id DESC LIMIT 200")?
        .query_map([project], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?, r.get::<_, String>(2)?, r.get::<_, String>(3)?, r.get::<_, String>(4)?, r.get::<_, Option<String>>(5)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    rows.into_iter().rev().map(|(id,file,role,text,images,request)| {
        Ok(json!({"id":id,"fileId":file,"role":role,"text":text,"images":serde_json::from_str::<Value>(&images)?,"requestId":request}))
    }).collect()
}
