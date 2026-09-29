use super::{blobs, media_store};
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};

pub fn migrate(db: &Connection) -> Result<()> {
    let done: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM database_migrations WHERE version=3)",
        [],
        |r| r.get(0),
    )?;
    if done {
        return Ok(());
    }
    let digests = {
        let mut stmt = db.prepare("SELECT digest FROM content_blobs WHERE media_digest IS NULL")?;
        stmt.query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for digest in digests {
        let text = blobs::read(db, &digest)?;
        let raw:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM event_content WHERE digest=?1 AND (pointer LIKE '%/data/value' OR pointer LIKE '%/b64_json')) OR EXISTS(SELECT 1 FROM job_content WHERE digest=?1 AND (pointer LIKE '%/data/value' OR pointer LIKE '%/b64_json' OR pointer LIKE '%/input_audio/data'))",[&digest],|r|r.get(0))?;
        if let Some((bytes, prefix)) =
            media_store::decode(&text, if raw { "/data/value" } else { "" })?
        {
            let tx = db.unchecked_transaction()?;
            let media = media_store::save(&tx, &bytes)?;
            tx.execute(
                "UPDATE content_blobs SET data=x'',media_digest=?1,media_prefix=?2 WHERE digest=?3",
                params![media, prefix, digest],
            )?;
            tx.commit()?;
        }
    }
    // Existing references are retained; extract inline media missed by the old size/path heuristic.
    for (table, column) in [("agent_events", "payload"), ("jobs", "data")] {
        let mut after = 0i64;
        loop {
            let row = db
                .query_row(
                    &format!(
                        "SELECT rowid,{column} FROM {table} WHERE rowid>?1 ORDER BY rowid LIMIT 1"
                    ),
                    [after],
                    |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()?;
            let Some((id, raw)) = row else { break };
            after = id;
            let mut value = serde_json::from_str(&raw)?;
            let tx = db.unchecked_transaction()?;
            let job = if table == "jobs" {
                tx.query_row("SELECT id FROM jobs WHERE rowid=?1", [id], |r| {
                    r.get::<_, String>(0)
                })?
            } else {
                String::new()
            };
            blobs::extend(
                &tx,
                if table == "jobs" {
                    blobs::Owner::Job(&job)
                } else {
                    blobs::Owner::Event(id)
                },
                &mut value,
            )?;
            let packed = value.to_string();
            if packed != raw {
                tx.execute(
                    &format!("UPDATE {table} SET {column}=?1 WHERE rowid=?2"),
                    params![packed, id],
                )?;
            }
            tx.commit()?;
        }
    }
    db.execute("INSERT INTO database_migrations(version) VALUES(3)", [])?;
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM; PRAGMA optimize;")?;
    Ok(())
}
