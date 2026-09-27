//! Project-scoped memory contract. Storage is independent from models and agents.
pub mod commands;
#[cfg(test)]
mod tests;
mod tool;
use anyhow::{Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
pub use tool::MemoryTool;

#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Entry {
    pub id: String,
    pub title: String,
    pub content: String,
    pub source: String,
    pub turn_id: Option<String>,
    pub updated: i64,
}
#[derive(Clone, Serialize, Deserialize, Debug, PartialEq)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Memory {
    pub revision: u64,
    pub enabled: bool,
    pub auto_update: bool,
    pub entries: Vec<Entry>,
}
impl Default for Memory {
    fn default() -> Self {
        Self {
            revision: 0,
            enabled: true,
            auto_update: true,
            entries: vec![],
        }
    }
}
pub trait MemoryBackend {
    fn recall(&self, project: &str) -> Result<Memory>;
    fn replace(&self, project: &str, memory: Memory) -> Result<Memory>;
}
pub struct SqliteMemory<'a>(pub &'a Connection);
pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS project_memory(project_id TEXT PRIMARY KEY REFERENCES projects(id) ON DELETE CASCADE,document TEXT NOT NULL);")?;
    Ok(())
}
impl MemoryBackend for SqliteMemory<'_> {
    fn recall(&self, project: &str) -> Result<Memory> {
        let exists: bool = self.0.query_row(
            "SELECT EXISTS(SELECT 1 FROM projects WHERE id=?1)",
            [project],
            |r| r.get(0),
        )?;
        ensure!(exists, "项目不存在");
        let raw: Option<String> = self
            .0
            .query_row(
                "SELECT document FROM project_memory WHERE project_id=?1",
                [project],
                |r| r.get(0),
            )
            .optional()?;
        Ok(match raw {
            Some(raw) => serde_json::from_str(&raw)?,
            None => Memory::default(),
        })
    }
    fn replace(&self, project: &str, mut memory: Memory) -> Result<Memory> {
        let tx = self.0.unchecked_transaction()?;
        let old = SqliteMemory(&tx).recall(project)?;
        ensure!(
            old.revision == memory.revision,
            "项目记忆已更新，请刷新后重试"
        );
        validate(&memory)?;
        if memory == old {
            return Ok(old);
        }
        memory.revision += 1;
        tx.execute("INSERT INTO project_memory VALUES(?1,?2) ON CONFLICT(project_id) DO UPDATE SET document=excluded.document", params![project, serde_json::to_string(&memory)?])?;
        tx.commit()?;
        Ok(memory)
    }
}
fn validate(memory: &Memory) -> Result<()> {
    ensure!(
        memory.entries.len() <= 40,
        "项目最多保存 40 条记忆，请合并已有内容"
    );
    let mut ids = std::collections::HashSet::new();
    let mut titles = std::collections::HashSet::new();
    for e in &memory.entries {
        ensure!(
            !e.id.is_empty() && e.id.len() <= 80 && ids.insert(&e.id),
            "记忆 ID 无效或重复"
        );
        ensure!(
            !e.title.trim().is_empty()
                && e.title.chars().count() <= 60
                && titles.insert(e.title.trim()),
            "记忆标题为空、重复或超过 60 字"
        );
        ensure!(
            !e.content.trim().is_empty() && e.content.chars().count() <= 1200,
            "每条记忆须为 1–1200 字"
        );
        ensure!(e.source.chars().count() <= 500, "记忆来源过长");
    }
    ensure!(
        memory
            .entries
            .iter()
            .map(|e| e.content.chars().count())
            .sum::<usize>()
            <= 12000,
        "项目记忆总长度不能超过 12000 字"
    );
    Ok(())
}
pub fn now() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}
/// Bounded recall: matching title/content first, then recent entries; full entries via tool.
pub fn context(memory: &Memory, prompt: &str) -> serde_json::Value {
    if !memory.enabled {
        return serde_json::json!({"enabled":false});
    }
    let query: Vec<char> = prompt.to_lowercase().chars().take(512).collect();
    let terms: Vec<String> = query.windows(2).map(|w| w.iter().collect()).collect();
    let mut entries: Vec<_> = memory.entries.iter().collect();
    entries.sort_by_cached_key(|e| {
        let text = format!("{} {}", e.title, e.content).to_lowercase();
        (
            std::cmp::Reverse(terms.iter().filter(|s| text.contains(s.as_str())).count()),
            std::cmp::Reverse(e.updated),
        )
    });
    let mut remaining = 6000;
    let recalled: Vec<_> = entries
        .into_iter()
        .filter_map(|e| {
            let v = serde_json::json!({"id":e.id,"title":e.title,"content":e.content});
            let cost = v.to_string().len();
            if cost > remaining {
                return None;
            }
            remaining -= cost;
            Some(v)
        })
        .collect();
    serde_json::json!({"enabled":true,"autoUpdate":memory.auto_update,"memoryRevision":memory.revision,"total":memory.entries.len(),"entries":recalled})
}
