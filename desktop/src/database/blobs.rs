//! Large JSON strings live once in a content-addressed table. References are trusted
//! relational rows, never magic objects interpreted from user/model JSON.
use anyhow::{Context, Result};
use flate2::{Compression, read::ZlibDecoder, write::ZlibEncoder};
use rusqlite::{Connection, params};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::io::{Read, Write};

#[derive(Clone, Copy)]
pub enum Owner<'a> {
    Event(i64),
    Job(&'a str),
}
impl Owner<'_> {
    fn key(&self) -> (&'static str, String) {
        match self {
            Self::Event(seq) => ("event_content", seq.to_string()),
            Self::Job(id) => ("job_content", id.to_string()),
        }
    }
}

// Caller owns the transaction containing the parent write and these references.
pub fn pack(db: &Connection, owner: Owner<'_>, value: &mut Value) -> Result<()> {
    let (table, key) = owner.key();
    db.execute(&format!("DELETE FROM {table} WHERE owner=?1"), [&key])?;
    extract(db, table, &key, "", value)
}
pub fn extend(db: &Connection, owner: Owner<'_>, value: &mut Value) -> Result<()> {
    let (table, key) = owner.key();
    extract(db, table, &key, "", value)
}
fn extract(
    db: &Connection,
    table: &str,
    key: &str,
    pointer: &str,
    value: &mut Value,
) -> Result<()> {
    match value {
        Value::String(text) => {
            let media = super::media_store::decode(text, pointer)?;
            if media.is_none() && (text.len() < 4096 || text.starts_with('/')) {
                return Ok(());
            }
            let digest = format!("{:x}", Sha256::digest(text.as_bytes()));
            let exists: bool = db.query_row(
                "SELECT EXISTS(SELECT 1 FROM content_blobs WHERE digest=?1)",
                [&digest],
                |r| r.get(0),
            )?;
            if !exists {
                if let Some((bytes, prefix)) = media {
                    let media_digest = super::media_store::save(db, &bytes)?;
                    db.execute("INSERT INTO content_blobs(digest,data,bytes,media_digest,media_prefix) VALUES(?1,x'',?2,?3,?4)",params![digest,text.len(),media_digest,prefix])?;
                } else {
                    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
                    encoder.write_all(text.as_bytes())?;
                    db.execute(
                        "INSERT INTO content_blobs(digest,data,bytes) VALUES(?1,?2,?3)",
                        params![digest, encoder.finish()?, text.len()],
                    )?;
                }
            }
            db.execute(
                &format!("INSERT INTO {table}(owner,pointer,digest) VALUES(?1,?2,?3)"),
                params![key, pointer, digest],
            )?;
            *value = Value::Null;
        }
        Value::Array(items) => {
            for (index, item) in items.iter_mut().enumerate() {
                extract(db, table, key, &format!("{pointer}/{index}"), item)?;
            }
        }
        Value::Object(items) => {
            for (name, item) in items {
                // Task lists and workers read these fields without loading request bodies.
                if pointer.is_empty()
                    && table == "job_content"
                    && !["input", "result", "outputs"].contains(&name.as_str())
                {
                    continue;
                }
                let name = name.replace('~', "~0").replace('/', "~1");
                extract(db, table, key, &format!("{pointer}/{name}"), item)?;
            }
        }
        _ => {}
    }
    Ok(())
}

/// Supports a SQL projection: references under omitted properties are not loaded.
pub fn hydrate(db: &Connection, owner: Owner<'_>, value: Value) -> Result<Value> {
    hydrate_content(db, owner, value, true)
}
/// Dependency scanning needs text references, never the binary media body.
pub fn text_event(db: &Connection, seq: i64, raw: &str) -> Result<Value> {
    hydrate_content(db, Owner::Event(seq), serde_json::from_str(raw)?, false)
}
fn hydrate_content(
    db: &Connection,
    owner: Owner<'_>,
    mut value: Value,
    media: bool,
) -> Result<Value> {
    let (table, key) = owner.key();
    let mut stmt = db.prepare(&format!(
        "SELECT pointer,r.digest FROM {table} r JOIN content_blobs b ON b.digest=r.digest WHERE owner=?1 AND (?2 OR b.media_digest IS NULL)"
    ))?;
    let refs = stmt.query_map(params![key, media], |r| {
        Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?))
    })?;
    for entry in refs {
        let (pointer, digest) = entry?;
        if let Some(slot) = value.pointer_mut(&pointer) {
            let text = read(db, &digest)?;
            *slot = Value::String(text);
        }
    }
    Ok(value)
}
pub fn read(db: &Connection, digest: &str) -> Result<String> {
    let (data, media, prefix): (Vec<u8>, Option<String>, Option<String>) = db.query_row(
        "SELECT data,media_digest,media_prefix FROM content_blobs WHERE digest=?1",
        [digest],
        |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    )?;
    let text = if let Some(media) = media {
        super::media_store::read(db, &media, prefix.as_deref().unwrap_or(""))?
    } else {
        let mut text = String::new();
        ZlibDecoder::new(data.as_slice())
            .read_to_string(&mut text)
            .context("Cannot decode stored content")?;
        text
    };
    anyhow::ensure!(
        format!("{:x}", Sha256::digest(text.as_bytes())) == digest,
        "Stored content checksum mismatch"
    );
    Ok(text)
}
pub fn event(db: &Connection, seq: i64, raw: &str) -> Result<Value> {
    hydrate(db, Owner::Event(seq), serde_json::from_str(raw)?)
}
pub fn collect(db: &Connection) -> Result<()> {
    db.execute("DELETE FROM content_blobs WHERE NOT EXISTS(SELECT 1 FROM event_content WHERE digest=content_blobs.digest) AND NOT EXISTS(SELECT 1 FROM job_content WHERE digest=content_blobs.digest)", [])?;
    super::media_store::collect(db)?;
    Ok(())
}
