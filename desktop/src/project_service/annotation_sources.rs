//! Recover legacy annotation provenance only from the application's structured
//! review message, never from a filename shared by ordinary uploads.
use anyhow::Result;
use rusqlite::Connection;
use serde_json::Value;

pub fn recover(db: &Connection, document: &mut Value) -> Result<()> {
    let Some(project) = document["id"].as_str() else {
        return Ok(());
    };
    let mut stmt = db.prepare("SELECT content FROM agent_messages WHERE project_id=?1 AND role='user' AND content LIKE '请根据图片批注修改这张已有图片，创建图片编辑任务。%' ORDER BY id")?;
    let messages = stmt.query_map([project], |r| r.get::<_, String>(0))?;
    for message in messages {
        let message = message?;
        let Some((source, guide)) = provenance(&message) else {
            continue;
        };
        let known = document["assets"]
            .as_array()
            .is_some_and(|a| a.iter().any(|a| a["id"] == source))
            || document["removedAssetIds"]
                .as_array()
                .is_some_and(|a| a.iter().any(|id| id == source));
        if !known {
            continue;
        }
        if let Some(assets) = document["assets"].as_array_mut() {
            for asset in assets {
                if asset["id"] == guide
                    && asset["attachmentOnly"] == true
                    && asset["annotationSourceId"].is_null()
                {
                    asset["annotationSourceId"] = Value::String(source.into());
                }
            }
        }
    }
    Ok(())
}
fn provenance(message: &str) -> Option<(&str, &str)> {
    let mut lines = message.lines();
    if lines.next()? != "请根据图片批注修改这张已有图片，创建图片编辑任务。"
    {
        return None;
    }
    let source = lines
        .next()?
        .strip_prefix("原图素材 ID：")?
        .split_once('（')?
        .0;
    let guide = lines
        .next()?
        .strip_prefix("内部批注说明图素材 ID：")?
        .split_once('；')?
        .0;
    if source.is_empty() || guide.is_empty() || source == guide {
        return None;
    }
    Some((source, guide))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn recovers_only_verified_review_attachments_in_this_project() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch(
            "CREATE TABLE agent_messages(id INTEGER,project_id TEXT,role TEXT,content TEXT);",
        )
        .unwrap();
        let message = "请根据图片批注修改这张已有图片，创建图片编辑任务。\n原图素材 ID：original（result.png），这是编辑底图。\n内部批注说明图素材 ID：guide；先读取批注位置。";
        db.execute(
            "INSERT INTO agent_messages VALUES(1,'p','user',?1)",
            [message],
        )
        .unwrap();
        let mut doc = json!({"id":"p","removedAssetIds":["original"],"assets":[{"id":"guide","attachmentOnly":true},{"id":"ordinary","name":"image-comments.png","attachmentOnly":true}]});
        recover(&db, &mut doc).unwrap();
        assert_eq!(doc["assets"][0]["annotationSourceId"], "original");
        assert!(doc["assets"][1]["annotationSourceId"].is_null());
        doc["id"] = json!("other");
        doc["assets"][0]
            .as_object_mut()
            .unwrap()
            .remove("annotationSourceId");
        recover(&db, &mut doc).unwrap();
        assert!(doc["assets"][0]["annotationSourceId"].is_null());
    }
}
