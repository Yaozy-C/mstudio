use rusqlite::Connection;
use serde_json::{Value, json};

pub fn read(db: &Connection, project: &str, id: &str) -> Value {
    let result = (|| -> anyhow::Result<Value> {
        let (seq, raw, index): (i64, String, usize) = db.query_row(
            "SELECT e.seq,e.payload,j.key FROM agent_events e,json_each(e.payload,'$.offloads') j WHERE e.project_id=?1 AND e.kind='image/offload' AND json_extract(j.value,'$.id')=?2 ORDER BY e.seq DESC LIMIT 1",
            rusqlite::params![project,id], |r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        )?;
        let mut value = crate::database::blobs::event(db, seq, &raw)?;
        Ok(value["offloads"][index]["image"].take())
    })();
    match result {
        Ok(image) => json!({"ok":true,"imageId":id,"__offloadedImage":image}),
        Err(error) => {
            let code = match error.downcast_ref::<rusqlite::Error>() {
                Some(rusqlite::Error::QueryReturnedNoRows) => "IMAGE_NOT_FOUND",
                Some(_) => "IMAGE_READ_FAILED",
                None => "IMAGE_CORRUPT",
            };
            json!({"error":error.to_string(),"code":code})
        }
    }
}
