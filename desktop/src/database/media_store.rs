//! Immutable local media snapshots. SQLite stores only the digest and wire prefix.
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};
use std::{io::Write, path::PathBuf};

pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS stored_media(digest TEXT PRIMARY KEY,bytes INTEGER NOT NULL);",
    )?;
    let present: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_table_info('content_blobs') WHERE name='media_digest')",
        [],
        |r| r.get(0),
    )?;
    if !present {
        db.execute_batch("ALTER TABLE content_blobs ADD COLUMN media_digest TEXT REFERENCES stored_media(digest);
            ALTER TABLE content_blobs ADD COLUMN media_prefix TEXT;")?;
    }
    db.execute_batch(
        "CREATE INDEX IF NOT EXISTS content_blobs_media ON content_blobs(media_digest);",
    )?;
    Ok(())
}
fn root(db: &Connection) -> Result<PathBuf> {
    let file: String = db.query_row(
        "SELECT file FROM pragma_database_list WHERE name='main'",
        [],
        |r| r.get(0),
    )?;
    let fallback = std::path::Path::new(&file)
        .parent()
        .context("Media storage has no directory")?;
    Ok(crate::storage::load(db, fallback)?
        .directory
        .join("reference-media"))
}
fn path(db: &Connection, digest: &str) -> Result<PathBuf> {
    ensure!(
        digest.len() == 64 && digest.bytes().all(|b| b.is_ascii_hexdigit()),
        "Invalid media digest"
    );
    Ok(root(db)?.join(digest))
}
/// Returns decoded bytes and the exact prefix needed to reconstruct provider input.
pub fn decode(text: &str, pointer: &str) -> Result<Option<(Vec<u8>, String)>> {
    let (prefix, body) = if text.starts_with("data:") {
        let Some((header, body)) = text.split_once(',') else {
            return Ok(None);
        };
        if !header.ends_with(";base64") {
            return Ok(None);
        }
        (format!("{header},"), body)
    } else {
        let field = pointer.ends_with("/data/value")
            || pointer.ends_with("/b64_json")
            || pointer.ends_with("/input_audio/data");
        let image = text.starts_with("iVBORw0KGgo")
            || text.starts_with("/9j/")
            || text.starts_with("R0lGOD")
            || text.starts_with("UklGR");
        if !field && !image {
            return Ok(None);
        }
        (String::new(), text)
    };
    let Ok(bytes) = STANDARD.decode(body) else {
        return Ok(None);
    };
    if bytes.is_empty() || STANDARD.encode(&bytes) != body {
        return Ok(None);
    }
    Ok(Some((bytes, prefix)))
}
pub fn save(db: &Connection, bytes: &[u8]) -> Result<String> {
    let digest = format!("{:x}", Sha256::digest(bytes));
    let destination = path(db, &digest)?;
    ensure!(
        destination.parent().unwrap().parent().unwrap().is_dir(),
        "Local media storage is unavailable"
    );
    std::fs::create_dir_all(destination.parent().unwrap())?;
    ensure!(
        !std::fs::symlink_metadata(destination.parent().unwrap())?
            .file_type()
            .is_symlink(),
        "Media directory cannot be a symlink"
    );
    if destination.exists() {
        ensure!(
            !std::fs::symlink_metadata(&destination)?
                .file_type()
                .is_symlink(),
            "Media file cannot be a symlink"
        );
        ensure!(
            std::fs::read(&destination)? == bytes,
            "Local media checksum mismatch"
        );
    } else {
        let temporary = destination.with_extension(format!("{}.tmp", mstudio::media::id()));
        let result = (|| -> Result<()> {
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temporary)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            std::fs::rename(&temporary, &destination)?;
            std::fs::File::open(destination.parent().unwrap())?.sync_all()?;
            Ok(())
        })();
        if result.is_err() {
            let _ = std::fs::remove_file(&temporary);
        }
        result?;
    }
    db.execute(
        "DELETE FROM pending_file_deletions WHERE path=?1",
        [destination.to_string_lossy().as_ref()],
    )?;
    db.execute(
        "INSERT OR IGNORE INTO stored_media VALUES(?1,?2)",
        params![digest, bytes.len()],
    )?;
    Ok(digest)
}
pub fn read(db: &Connection, digest: &str, prefix: &str) -> Result<String> {
    let file = path(db, digest)?;
    ensure!(
        !std::fs::symlink_metadata(&file)?.file_type().is_symlink(),
        "Media file cannot be a symlink"
    );
    let bytes = std::fs::read(&file)
        .with_context(|| format!("Referenced media is missing: {}", file.display()))?;
    ensure!(
        format!("{:x}", Sha256::digest(&bytes)) == digest,
        "Local media checksum mismatch"
    );
    Ok(format!("{prefix}{}", STANDARD.encode(bytes)))
}
/// Queue deletion transactionally; disk deletion happens after commit. Reuse cancels it.
pub fn collect(db: &Connection) -> Result<()> {
    let digests = {
        let mut stmt=db.prepare("SELECT digest FROM stored_media WHERE NOT EXISTS(SELECT 1 FROM content_blobs WHERE media_digest=stored_media.digest)")?;
        stmt.query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for digest in digests {
        db.execute(
            "INSERT OR IGNORE INTO pending_file_deletions VALUES('media-cache',?1)",
            [path(db, &digest)?.to_string_lossy().as_ref()],
        )?;
        db.execute("DELETE FROM stored_media WHERE digest=?1", [digest])?;
    }
    Ok(())
}
/// A rolled-back write may leave a complete unreferenced file; startup safely reclaims it.
pub fn sweep(db: &Connection) -> Result<()> {
    let directory = root(db)?;
    if !directory.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(&directory)? {
        let entry = entry?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if name.len() != 64 || !name.bytes().all(|b| b.is_ascii_hexdigit()) {
            continue;
        }
        let used: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM stored_media WHERE digest=?1)",
            [name.as_ref()],
            |r| r.get(0),
        )?;
        if !used {
            db.execute(
                "INSERT OR IGNORE INTO pending_file_deletions VALUES('media-cache',?1)",
                [entry.path().to_string_lossy().as_ref()],
            )?;
        }
    }
    Ok(())
}
