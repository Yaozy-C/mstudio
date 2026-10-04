use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::Value;
pub fn read(db: &Connection, project: &str, execution: &str) -> Result<Option<Value>> {
    let raw: Option<String> = db
        .query_row(
            "SELECT receipt FROM project_executions WHERE project_id=?1 AND execution_id=?2",
            params![project, execution],
            |r| r.get(0),
        )
        .optional()?;
    raw.map(|raw| serde_json::from_str(&raw).map_err(Into::into))
        .transpose()
}
