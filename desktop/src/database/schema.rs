use anyhow::Result;
use rusqlite::Connection;

pub fn init(db: &Connection) -> Result<()> {
    let linked: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM pragma_foreign_key_list('jobs') WHERE \"table\"='projects')",
        [],
        |r| r.get(0),
    )?;
    if !linked {
        let tx = db.unchecked_transaction()?;
        tx.execute_batch("CREATE TABLE jobs_normalized(id TEXT PRIMARY KEY,project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,data TEXT NOT NULL);
            INSERT INTO jobs_normalized(rowid,id,project_id,data) SELECT rowid,id,project_id,data FROM jobs;
            DROP TABLE jobs;
            ALTER TABLE jobs_normalized RENAME TO jobs;")?;
        tx.commit()?;
    }
    db.execute_batch("
        CREATE TABLE IF NOT EXISTS database_migrations(version INTEGER PRIMARY KEY,applied INTEGER NOT NULL DEFAULT(unixepoch()));
        CREATE TABLE IF NOT EXISTS content_blobs(digest TEXT PRIMARY KEY,data BLOB NOT NULL,bytes INTEGER NOT NULL CHECK(bytes>=0));
        CREATE TABLE IF NOT EXISTS event_content(
            owner INTEGER NOT NULL REFERENCES agent_events(seq) ON DELETE CASCADE,
            pointer TEXT NOT NULL,digest TEXT NOT NULL REFERENCES content_blobs(digest),
            PRIMARY KEY(owner,pointer));
        CREATE TABLE IF NOT EXISTS job_content(
            owner TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
            pointer TEXT NOT NULL,digest TEXT NOT NULL REFERENCES content_blobs(digest),
            PRIMARY KEY(owner,pointer));
        CREATE INDEX IF NOT EXISTS event_content_digest ON event_content(digest);
        CREATE INDEX IF NOT EXISTS job_content_digest ON job_content(digest);
        CREATE INDEX IF NOT EXISTS agent_events_turn ON agent_events(project_id,turn_id,seq);
        CREATE INDEX IF NOT EXISTS agent_events_kind ON agent_events(project_id,kind,seq);
        CREATE INDEX IF NOT EXISTS agent_events_call ON agent_events(project_id,turn_id,json_extract(payload,'$.callId'),seq) WHERE kind='tool/result';
        CREATE INDEX IF NOT EXISTS agent_messages_turn ON agent_messages(project_id,json_extract(attribution,'$.turnId'),role);
        CREATE INDEX IF NOT EXISTS jobs_project ON jobs(project_id);
        CREATE INDEX IF NOT EXISTS jobs_connection_status ON jobs(json_extract(data,'$.connectionId'),json_extract(data,'$.status'));
        CREATE INDEX IF NOT EXISTS project_assets_asset ON project_assets(asset_id);
        CREATE INDEX IF NOT EXISTS project_files_path ON project_files(path);
        CREATE INDEX IF NOT EXISTS subagent_inbox_pending ON subagent_inbox(child_id,consumed,seq);
        CREATE INDEX IF NOT EXISTS subagent_last_turn ON subagent_runs(project_id,last_turn);
    ")?;
    Ok(())
}

/// One parent per transaction bounds memory and makes interruption safely resumable.
/// An already packed parent has no large strings; its references must be retained.
pub fn migrate(db: &Connection) -> Result<()> {
    let done: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM database_migrations WHERE version=1)",
        [],
        |r| r.get(0),
    )?;
    if done {
        return Ok(());
    }
    for (table, column, refs) in [
        ("agent_events", "payload", "event_content"),
        ("jobs", "data", "job_content"),
    ] {
        let mut after = 0i64;
        loop {
            use rusqlite::OptionalExtension;
            let next = db
                .query_row(
                    &format!(
                        "SELECT rowid,{column} FROM {table} WHERE rowid>?1 ORDER BY rowid LIMIT 1"
                    ),
                    [after],
                    |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)),
                )
                .optional()?;
            let Some((rowid, raw)) = next else {
                break;
            };
            after = rowid;
            if raw.len() < 4096 {
                continue;
            }
            let tx = db.unchecked_transaction()?;
            let id: String = if table == "jobs" {
                tx.query_row("SELECT id FROM jobs WHERE rowid=?1", [rowid], |r| r.get(0))?
            } else {
                rowid.to_string()
            };
            let packed: bool = tx.query_row(
                &format!("SELECT EXISTS(SELECT 1 FROM {refs} WHERE owner=?1)"),
                [&id],
                |r| r.get(0),
            )?;
            if !packed {
                let mut value = serde_json::from_str(&raw)?;
                let owner = if table == "jobs" {
                    super::blobs::Owner::Job(&id)
                } else {
                    super::blobs::Owner::Event(rowid)
                };
                super::blobs::pack(&tx, owner, &mut value)?;
                tx.execute(
                    &format!("UPDATE {table} SET {column}=?1 WHERE rowid=?2"),
                    rusqlite::params![value.to_string(), rowid],
                )?;
            }
            tx.commit()?;
        }
    }
    db.execute("INSERT INTO database_migrations(version) VALUES(1)", [])?;
    db.execute_batch("PRAGMA wal_checkpoint(TRUNCATE); VACUUM; PRAGMA optimize;")?;
    Ok(())
}
