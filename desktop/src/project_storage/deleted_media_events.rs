use anyhow::Result;
use rusqlite::{Connection, params};
use serde_json::{Value, json};
use std::collections::HashSet;

// Retire binary history without leaving malformed image blocks in replayable
// messages. Keep the conversation text and tool-call pairing intact.
pub fn retire(db: &Connection, project: &str, digests: &HashSet<String>) -> Result<()> {
    for digest in digests {
        let rows = db.prepare("SELECT e.seq,e.payload,c.pointer FROM event_content c JOIN agent_events e ON e.seq=c.owner WHERE e.project_id=?2 AND (c.digest=?1 OR c.digest IN (SELECT digest FROM content_blobs WHERE media_digest=(SELECT media_digest FROM content_blobs WHERE digest=?1)))")?
            .query_map(params![digest,project], |r| Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?)))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        for (seq, _, pointer) in rows {
            // A previous pointer may already have replaced a whole content block.
            let raw: String = db.query_row(
                "SELECT payload FROM agent_events WHERE seq=?1",
                [seq],
                |r| r.get(0),
            )?;
            let mut event: Value = serde_json::from_str(&raw)?;
            let mut prefix = pointer.clone();
            let mut replacement = json!("[素材已删除]");
            let mut ancestor = pointer.as_str();
            while let Some((parent, _)) = ancestor.rsplit_once('/') {
                ancestor = parent;
                if let Some(value) = event.pointer(ancestor) {
                    if matches!(value["type"].as_str(), Some("image" | "audio" | "document")) {
                        prefix = ancestor.into();
                        replacement = json!({"type":"text","text":"[素材已删除]"});
                        break;
                    }
                    if ancestor.ends_with("/image") {
                        prefix = ancestor.into();
                        replacement = json!({"deleted":true});
                        break;
                    }
                }
                if ancestor.is_empty() {
                    break;
                }
            }
            if let Some(value) = event.pointer_mut(&prefix) {
                *value = replacement;
            }
            db.execute(
                "UPDATE agent_events SET payload=?1 WHERE seq=?2",
                params![event.to_string(), seq],
            )?;
            db.execute("DELETE FROM event_content WHERE owner=?1 AND (pointer=?2 OR substr(pointer,1,length(?2)+1)=?2||'/')", params![seq,prefix])?;
        }
    }
    Ok(())
}

fn scrub_file_images(value: &mut Value, paths: &HashSet<String>) -> bool {
    let source = value["data"]["value"].as_str().unwrap_or("");
    if paths.contains(source.strip_prefix("file://").unwrap_or(source)) {
        *value = if value["type"] == "image" {
            json!({"type":"text","text":"[素材已删除]"})
        } else {
            json!({"deleted":true})
        };
        return true;
    }
    let mut changed = false;
    match value {
        Value::Array(items) => {
            for item in items {
                changed |= scrub_file_images(item, paths);
            }
        }
        Value::Object(items) => {
            for item in items.values_mut() {
                changed |= scrub_file_images(item, paths);
            }
        }
        _ => {}
    }
    changed
}

pub fn retire_files(db: &Connection, project: &str, paths: &HashSet<String>) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let events = db
        .prepare("SELECT seq,payload FROM agent_events WHERE project_id=?1")?
        .query_map([project], |r| {
            Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (seq, raw) in events {
        let mut event: Value = serde_json::from_str(&raw)?;
        if scrub_file_images(&mut event, paths) {
            db.execute(
                "UPDATE agent_events SET payload=?1 WHERE seq=?2",
                params![event.to_string(), seq],
            )?;
        }
    }
    Ok(())
}
