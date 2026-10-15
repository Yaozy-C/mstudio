//! Media survives until a subsequent assistant response, then becomes a durable reference.
use super::{
    Host,
    session::{Session, consumed_prefix},
};
use rig_core::message::{AssistantContent, Message, ToolResultContent, UserContent};
use serde_json::{Value, json};

fn offload(content: &UserContent, offloads: &mut Vec<Value>) -> String {
    let image = matches!(content, UserContent::Image(_));
    let id = format!(
        "{}-{}",
        if image { "image" } else { "media" },
        mstudio::media::id()
    );
    if let UserContent::Image(value) = content {
        offloads.push(json!({"id":id,"image":value}));
    } else {
        offloads.push(json!({"id":id,"content":content}));
    }
    let tool = if image {
        "mstudio_reopen_image(imageId"
    } else {
        "mstudio_reopen_media(mediaId"
    };
    format!(
        "[Earlier media offloaded after model processing; reference {id}. Reopen only when needed with {tool}=\"{id}\").]"
    )
}

pub fn offload_consumed_media(session: &mut Session, host: &impl Host) -> Result<usize, String> {
    let mut total = 0;
    for index in 0..consumed_prefix(&session.messages) {
        let mut message = session.messages[index].clone();
        let mut offloads = Vec::new();
        match &mut message {
            Message::User { content } => {
                for part in content {
                    match part {
                        UserContent::Image(_)
                        | UserContent::Audio(_)
                        | UserContent::Video(_)
                        | UserContent::Document(_) => {
                            *part = UserContent::text(offload(part, &mut offloads));
                        }
                        UserContent::Text(text) if super::context_source::is_attachment(text) => {
                            *part = UserContent::text(offload(part, &mut offloads));
                        }
                        UserContent::ToolResult(result) => {
                            for part in &mut result.content {
                                if let ToolResultContent::Image(image) = part {
                                    *part = ToolResultContent::text(offload(
                                        &UserContent::Image(image.clone()),
                                        &mut offloads,
                                    ));
                                }
                            }
                        }
                        _ => {}
                    }
                }
            }
            Message::Assistant(value) => {
                for part in &mut value.content {
                    if let AssistantContent::Image(image) = part {
                        *part = AssistantContent::text(offload(
                            &UserContent::Image(image.clone()),
                            &mut offloads,
                        ));
                    }
                }
            }
            _ => {}
        }
        if offloads.is_empty() {
            continue;
        }
        total += offloads.len();
        host.record(
            "media/offload",
            json!({"messageIndex":index,"message":message,"offloads":offloads}),
        )?;
        session.messages[index] = message;
    }
    Ok(total)
}

pub fn read(
    db: &rusqlite::Connection,
    project: &str,
    id: &str,
    model: &crate::assistant::config::Profile,
) -> Value {
    let read = (|| -> anyhow::Result<UserContent> {
        let (seq, raw, index): (i64, String, usize) = db.query_row(
            "SELECT e.seq,e.payload,j.key FROM agent_events e,json_each(e.payload,'$.offloads') j WHERE e.project_id=?1 AND e.kind='media/offload' AND json_extract(j.value,'$.id')=?2 ORDER BY e.seq DESC LIMIT 1",
            rusqlite::params![project,id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        )?;
        let mut event = crate::database::blobs::event(db, seq, &raw)?;
        let content: UserContent =
            serde_json::from_value(event["offloads"][index]["content"].take())?;
        anyhow::ensure!(
            match &content {
                UserContent::Audio(_) => model.inputs.audio,
                UserContent::Video(_) => model.inputs.video,
                UserContent::Document(_) => model.inputs.document,
                UserContent::Text(text) => super::context_source::is_attachment(text),
                _ => false,
            },
            "Current model does not support this media input"
        );
        Ok(content)
    })();
    match read {
        Ok(content) => json!({"ok":true,"mediaId":id,"__offloadedMedia":content}),
        Err(_) => {
            json!({"error":"Media reference is unavailable in this project or unsupported by this model","code":"MEDIA_UNAVAILABLE"})
        }
    }
}
