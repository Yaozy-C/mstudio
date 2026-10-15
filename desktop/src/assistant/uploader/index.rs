use super::{Error, REFRESH_MARGIN, Result};
use rusqlite::{Connection, OptionalExtension, params};

pub struct Uploaded {
    pub id: String,
    pub bytes: u64,
    pub expires_at: i64,
}

pub(super) fn init(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS provider_uploads(
        scope TEXT NOT NULL,variant TEXT NOT NULL,file_id TEXT NOT NULL,
        bytes INTEGER NOT NULL,expires_at INTEGER NOT NULL,
        PRIMARY KEY(scope,variant));",
    )
    .map_err(Error::instance)
}

pub(super) fn get(db: &Connection, scope: &str, variant: &str, now: i64) -> Result<Option<String>> {
    db.query_row(
        "SELECT file_id FROM provider_uploads WHERE scope=?1 AND variant=?2 AND expires_at>?3",
        params![scope, variant, now + REFRESH_MARGIN],
        |r| r.get(0),
    )
    .optional()
    .map_err(Error::instance)
}

pub(super) fn put(db: &Connection, scope: &str, variant: &str, file: &Uploaded) -> Result<()> {
    db.execute("INSERT INTO provider_uploads(scope,variant,file_id,bytes,expires_at) VALUES(?1,?2,?3,?4,?5)
        ON CONFLICT(scope,variant) DO UPDATE SET file_id=excluded.file_id,bytes=excluded.bytes,expires_at=excluded.expires_at",
        params![scope, variant, file.id, file.bytes, file.expires_at]).map_err(Error::instance)?;
    Ok(())
}

pub(super) fn remove(db: &Connection, scope: &str, variant: &str, id: &str) -> Result<()> {
    db.execute(
        "DELETE FROM provider_uploads WHERE scope=?1 AND variant=?2 AND file_id=?3",
        params![scope, variant, id],
    )
    .map_err(Error::instance)?;
    Ok(())
}
