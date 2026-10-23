//! Dedicated previsualization Agent with its own scoped files and session.
mod agent;
pub(crate) mod chat;
mod chat_context;
mod editor;
mod editor_commands;
#[cfg(test)]
mod editor_loop_tests;
mod editor_schema;
#[cfg(test)]
mod editor_tests;
mod export_cleanup;
mod export_references;
mod export_store;
#[cfg(test)]
mod export_store_tests;
pub(crate) mod files;
pub(crate) mod media;
mod messages;
#[cfg(test)]
mod messages_tests;
pub(crate) mod references;
pub(crate) mod render;
pub(crate) mod runtime;
#[cfg(test)]
mod tests;

pub use chat::Chats;
pub use render::Renders;
pub use runtime::RuntimeState;

use anyhow::{Result, ensure};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Clone, Serialize, Deserialize, Debug)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct File {
    pub id: String,
    pub kind: String,
    pub name: String,
    pub description: String,
    pub code: String,
    pub revision: u64,
    pub asset_ids: Vec<String>,
    pub bindings: BTreeMap<String, String>,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
}

impl File {
    pub fn validate(&self) -> Result<()> {
        identifier(&self.id)?;
        ensure!(
            ["model", "shot"].contains(&self.kind.as_str()),
            "预演文件类型无效"
        );
        ensure!(!self.name.trim().is_empty(), "请给模型或镜头填写名称");
        ensure!(self.fps > 0, "预演帧率须大于零");
        ensure!(
            self.duration.is_finite() && self.duration >= 1.0 / self.fps as f64,
            "预演时长须至少一帧"
        );
        ensure!(
            self.width > 0
                && self.height > 0
                && self.width.is_multiple_of(2)
                && self.height.is_multiple_of(2),
            "预演宽高须为正偶数"
        );
        for id in &self.asset_ids {
            identifier(id)?;
        }
        Ok(())
    }
}
pub fn identifier(id: &str) -> Result<()> {
    ensure!(
        !id.is_empty()
            && id.len() <= 100
            && id
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'-' || c == b'_'),
        "预演标识无效"
    );
    Ok(())
}
pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS previs_files(project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,id TEXT NOT NULL,kind TEXT NOT NULL,data TEXT NOT NULL,PRIMARY KEY(project_id,id));
    CREATE TABLE IF NOT EXISTS previs_messages(id INTEGER PRIMARY KEY,project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,file_id TEXT NOT NULL,role TEXT NOT NULL,text TEXT NOT NULL,images TEXT NOT NULL,request_id TEXT NOT NULL);
    CREATE TABLE IF NOT EXISTS previs_exports(project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,id TEXT NOT NULL,data TEXT NOT NULL,PRIMARY KEY(project_id,id));")?;
    media::init(db)?;
    Ok(())
}
