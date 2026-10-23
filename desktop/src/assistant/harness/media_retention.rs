//! Budget-driven omissions preserve saved media and stable recovery references.
use super::{
    Host,
    session::{Session, consumed_prefix},
};
use rig_core::message::{Message, ToolResultContent, UserContent};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};

pub fn valid_reference(id: &str, prefix: &str) -> bool {
    id.strip_prefix(prefix).is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
    })
}

fn offload(content: &UserContent, offloads: &mut Vec<Value>) -> String {
    let image = matches!(content, UserContent::Image(_));
    let id = format!(
        "{}-{}",
        if image { "image" } else { "media" },
        format_args!(
            "{:x}",
            Sha256::digest(serde_json::to_vec(content).expect("media serializes"))
        )
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
        "[Earlier media omitted to fit the request budget; saved reference {id}. Reopen for a specific missing detail with {tool}=\"{id}\").]"
    )
}

pub fn offload_oldest_media(
    session: &mut Session,
    host: &impl Host,
    count: usize,
) -> Result<usize, String> {
    offload_oldest(session, host, count, false)
}

pub fn offload_oldest_images(
    session: &mut Session,
    host: &impl Host,
    count: usize,
) -> Result<usize, String> {
    offload_oldest(session, host, count, true)
}

fn offload_oldest(
    session: &mut Session,
    host: &impl Host,
    mut remaining: usize,
    images_only: bool,
) -> Result<usize, String> {
    if host.token().is_cancelled() {
        return Err(crate::app_error::cancelled());
    }
    let mut total = 0;
    for index in 0..consumed_prefix(&session.messages) {
        if remaining == 0 {
            break;
        }
        let mut message = session.messages[index].clone();
        let mut offloads = Vec::new();
        if let Message::User { content } = &mut message {
            for part in content {
                if remaining == 0 {
                    break;
                }
                match part {
                    UserContent::Image(_) => {
                        *part = UserContent::text(offload(part, &mut offloads));
                        remaining -= 1;
                    }
                    UserContent::Audio(_) | UserContent::Video(_) | UserContent::Document(_)
                        if !images_only =>
                    {
                        *part = UserContent::text(offload(part, &mut offloads));
                        remaining -= 1;
                    }
                    UserContent::Text(text)
                        if !images_only && super::context_source::is_attachment(text) =>
                    {
                        *part = UserContent::text(offload(part, &mut offloads));
                        remaining -= 1;
                    }
                    UserContent::ToolResult(result) => {
                        for part in &mut result.content {
                            if remaining == 0 {
                                break;
                            }
                            if let ToolResultContent::Image(image) = part {
                                *part = ToolResultContent::text(offload(
                                    &UserContent::Image(image.clone()),
                                    &mut offloads,
                                ));
                                remaining -= 1;
                            }
                        }
                    }
                    _ => {}
                }
            }
        }
        if offloads.is_empty() {
            continue;
        }
        if host.token().is_cancelled() {
            return Err(crate::app_error::cancelled());
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
    if !valid_reference(id, "media-") {
        return json!({"error":"Use a content-hash mediaId from an offload notice","code":"INVALID_ARGS"});
    }
    let read = (|| -> anyhow::Result<UserContent> {
        let (seq, raw, index): (i64, String, usize) = db.query_row(
            "SELECT e.seq,e.payload,j.key FROM agent_events e,json_each(e.payload,'$.offloads') j WHERE e.project_id=?1 AND e.kind='media/offload' AND json_extract(j.value,'$.id')=?2 ORDER BY e.seq DESC LIMIT 1",
            rusqlite::params![project,id], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?)),
        )?;
        let mut event = crate::database::blobs::event(db, seq, &raw)?;
        let content: UserContent =
            serde_json::from_value(event["offloads"][index]["content"].take())?;
        Ok(content)
    })();
    match read {
        Ok(content) => {
            let supported = match &content {
                UserContent::Audio(_) => model.inputs.audio,
                UserContent::Video(_) => model.inputs.video,
                UserContent::Document(_) => model.inputs.document,
                UserContent::Text(text) => super::context_source::is_attachment(text),
                _ => false,
            };
            if supported {
                json!({"ok":true,"mediaId":id,"__offloadedMedia":content})
            } else {
                json!({"error":"Current model does not support this media input","code":"MEDIA_UNSUPPORTED"})
            }
        }
        Err(error) => super::read_failure::result(error, "MEDIA_NOT_FOUND", "MEDIA_READ_FAILED"),
    }
}
