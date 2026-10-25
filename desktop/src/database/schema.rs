use anyhow::Result;
use rusqlite::Connection;

pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch("
        CREATE TABLE IF NOT EXISTS image_processing_results(
            asset_id TEXT PRIMARY KEY REFERENCES assets(id) ON DELETE CASCADE,
            project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
            input_asset_id TEXT NOT NULL,data TEXT NOT NULL,
            created INTEGER NOT NULL DEFAULT(unixepoch()));
        CREATE INDEX IF NOT EXISTS image_processing_project ON image_processing_results(project_id);
        CREATE TABLE IF NOT EXISTS stored_media(digest TEXT PRIMARY KEY,bytes INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS content_blobs(digest TEXT PRIMARY KEY,data BLOB NOT NULL,bytes INTEGER NOT NULL CHECK(bytes>=0),media_digest TEXT REFERENCES stored_media(digest),media_prefix TEXT);
        CREATE TABLE IF NOT EXISTS event_content(
            owner INTEGER NOT NULL REFERENCES agent_events(seq) ON DELETE CASCADE,
            pointer TEXT NOT NULL,digest TEXT NOT NULL REFERENCES content_blobs(digest),
            PRIMARY KEY(owner,pointer));
        CREATE TABLE IF NOT EXISTS job_content(
            owner TEXT NOT NULL REFERENCES jobs(id) ON DELETE CASCADE,
            pointer TEXT NOT NULL,digest TEXT NOT NULL REFERENCES content_blobs(digest),
            PRIMARY KEY(owner,pointer));
        CREATE INDEX IF NOT EXISTS content_blobs_media ON content_blobs(media_digest);
        CREATE UNIQUE INDEX IF NOT EXISTS agent_events_usage ON agent_events(project_id,turn_id) WHERE kind='turn/usage';
        CREATE INDEX IF NOT EXISTS event_content_digest ON event_content(digest);
        CREATE INDEX IF NOT EXISTS job_content_digest ON job_content(digest);
        CREATE INDEX IF NOT EXISTS agent_events_turn ON agent_events(project_id,turn_id,seq);
        CREATE INDEX IF NOT EXISTS agent_events_kind ON agent_events(project_id,kind,seq);
        CREATE INDEX IF NOT EXISTS agent_events_call ON agent_events(project_id,turn_id,json_extract(payload,'$.callId'),seq) WHERE kind='tool/result';
        CREATE INDEX IF NOT EXISTS agent_messages_turn ON agent_messages(project_id,json_extract(attribution,'$.turnId'),role);
        CREATE INDEX IF NOT EXISTS jobs_project ON jobs(project_id);
        CREATE INDEX IF NOT EXISTS projects_updated ON projects(updated DESC);
        CREATE INDEX IF NOT EXISTS jobs_status ON jobs(json_extract(data,'$.status'));
        CREATE INDEX IF NOT EXISTS jobs_connection_status ON jobs(json_extract(data,'$.connectionId'),json_extract(data,'$.status'));
        CREATE INDEX IF NOT EXISTS project_assets_asset ON project_assets(asset_id);
        CREATE INDEX IF NOT EXISTS project_files_path ON project_files(path);
        CREATE INDEX IF NOT EXISTS subagent_inbox_pending ON subagent_inbox(child_id,consumed,seq);
        CREATE INDEX IF NOT EXISTS subagent_last_turn ON subagent_runs(project_id,last_turn);
    ")?;
    Ok(())
}
