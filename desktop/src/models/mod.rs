pub mod capabilities;
pub mod codex_connection;
pub mod commands;
#[cfg(test)]
mod connection_tests;
pub mod connections;
pub mod media;
mod storage;
#[cfg(test)]
mod tests;

use crate::assistant::config::Profile;
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Model {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub connection_id: Option<String>,
    #[serde(flatten)]
    pub profile: Profile,
    #[serde(default, skip_deserializing)]
    pub has_key: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Catalog {
    pub profiles: Vec<Model>,
    pub default_id: Option<String>,
    pub selected_id: Option<String>,
}

pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS model_profiles(id TEXT PRIMARY KEY,data TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS model_credentials(id TEXT PRIMARY KEY REFERENCES model_profiles(id) ON DELETE CASCADE,endpoint TEXT NOT NULL,api_key TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS agent_model_preferences(project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,model_id TEXT REFERENCES model_profiles(id) ON DELETE SET NULL);")?;
    connections::init(db)?;
    capabilities::migrate(db)?;
    Ok(())
}

pub(crate) fn setting(db: &Connection, key: &str) -> Result<Option<String>> {
    Ok(db
        .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
            r.get(0)
        })
        .optional()?)
}

pub use storage::{catalog, resolve};
