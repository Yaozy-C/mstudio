mod access;
#[cfg(test)]
mod audit_tests;
pub mod blobs;
pub(crate) mod event_retention;
#[cfg(test)]
mod event_retention_tests;
#[cfg(test)]
mod history_audit_tests;
pub(crate) mod history_cleanup;
#[cfg(test)]
mod history_cleanup_tests;
#[cfg(test)]
mod media_audit_tests;
mod media_migration;
pub(crate) mod media_store;
#[cfg(test)]
pub(crate) mod media_tests;
mod schema;
pub(crate) mod session_checkpoint;
#[cfg(test)]
mod session_checkpoint_tests;
#[cfg(test)]
mod tests;
pub(crate) mod turn_usage;
use anyhow::Result;
use mstudio::model::Asset;
use rusqlite::Connection;
#[cfg(test)]
use rusqlite::{OptionalExtension, params};
use serde_json::{Value, json};
use std::{path::PathBuf, sync::Mutex};

pub struct Store {
    #[cfg(test)]
    pub root: PathBuf,
    pub db: Mutex<Connection>,
    pub files: std::sync::Arc<tokio::sync::RwLock<()>>,
    pub imports: Mutex<()>,
    pub project_events: tokio::sync::broadcast::Sender<String>,
    pub location: std::sync::RwLock<crate::storage::Location>,
}
impl Store {
    pub fn open(root: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&root)?;
        let db = Connection::open(root.join("mstudio.sqlite3"))?;
        db.execute_batch("PRAGMA journal_mode=WAL; PRAGMA foreign_keys=ON; PRAGMA busy_timeout=5000;
        CREATE TABLE IF NOT EXISTS projects(id TEXT PRIMARY KEY,name TEXT NOT NULL,document TEXT NOT NULL,updated INTEGER NOT NULL);
        CREATE TABLE IF NOT EXISTS assets(id TEXT PRIMARY KEY,data TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS settings(key TEXT PRIMARY KEY,value TEXT NOT NULL);
        CREATE TABLE IF NOT EXISTS jobs(id TEXT PRIMARY KEY,project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,data TEXT NOT NULL);")?;
        let location = crate::storage::load(&db, &root)?;
        db.execute(
            "INSERT OR IGNORE INTO settings(key,value) VALUES('file-storage',?1)",
            [serde_json::to_string(&location)?],
        )?;
        crate::assistant::history::init(&db)?;
        crate::assistant::result_check::init(&db)?;
        crate::assistant::history::interrupt_pending(&db)?;
        crate::assistant::journal::init(&db)?;
        crate::models::init(&db)?;
        crate::model_adapters::prompt_rules::init(&db)?;
        crate::project_storage::init(&db)?;
        crate::project_service::init(&db)?;
        crate::asset_library::init(&db)?;
        crate::project_storage::direct_references::migrate(&db, &root)?;
        crate::job_recovery::recover_interrupted_submissions(&db)?;
        schema::init(&db)?;
        media_store::init(&db)?;
        schema::migrate(&db)?;
        event_retention::migrate(&db)?;
        media_migration::migrate(&db)?;
        history_cleanup::startup(&db)?;
        media_store::sweep(&db)?;
        if let Err(error) = crate::project_storage::resume_cleanup(&db, &location.directory) {
            eprintln!("{error}");
        }
        Ok(Self {
            #[cfg(test)]
            root,
            db: Mutex::new(db),
            files: Default::default(),
            imports: Mutex::new(()),
            project_events: tokio::sync::broadcast::channel(64).0,
            location: std::sync::RwLock::new(location),
        })
    }
    pub fn media_root(&self) -> PathBuf {
        self.location.read().unwrap().directory.clone()
    }
    pub fn project_changed(&self, id: &str) {
        let _ = self.project_events.send(id.to_owned());
    }
    pub fn normalize_paths(&self, value: &mut Value) {
        let location = self.location.read().unwrap();
        for previous in &location.previous {
            crate::storage::rewrite(value, previous, &location.directory);
        }
    }
    pub fn assets(&self) -> Result<Vec<Asset>> {
        let db = self.db.lock().unwrap();
        let mut stmt = db.prepare("SELECT data FROM assets")?;
        let values = stmt
            .query_map([], |r| r.get::<_, String>(0))?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        values
            .iter()
            .map(|s| {
                let mut asset: Asset = serde_json::from_str(s)?;
                asset.missing = !std::path::Path::new(&asset.path).is_file();
                Ok(asset)
            })
            .collect()
    }
    #[cfg(test)]
    pub fn setting(&self, key: &str) -> Result<String> {
        Ok(self
            .db
            .lock()
            .unwrap()
            .query_row("SELECT value FROM settings WHERE key=?1", [key], |r| {
                r.get(0)
            })
            .optional()?
            .unwrap_or_default())
    }
    #[cfg(test)]
    pub fn set_setting(&self, key: &str, value: &str) -> Result<()> {
        self.db.lock().unwrap().execute("INSERT INTO settings VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", params![key, value])?;
        Ok(())
    }
    pub fn projects(&self) -> Result<Vec<Value>> {
        let db = self.db.lock().unwrap();
        let generated = crate::asset_library::generated_ids(&db)?;
        let mut stmt =
            db.prepare("SELECT id,name,updated,document FROM projects ORDER BY updated DESC")?;
        let rows = stmt
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, i64>(2)?,
                    r.get::<_, String>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?;
        rows.into_iter()
            .map(|(id, name, updated, document)| {
                let mut document: Value = serde_json::from_str(&document)?;
                if let Some(assets) = document["assets"].as_array_mut() {
                    for asset in assets {
                        asset["missing"] = json!(
                            !asset["path"]
                                .as_str()
                                .is_some_and(|p| std::path::Path::new(p).is_file())
                        );
                        if asset["id"]
                            .as_str()
                            .is_some_and(|id| generated.contains(id))
                        {
                            asset["generated"] = json!(true);
                        }
                    }
                }
                Ok(json!({"id":id,"name":name,"updated":updated,"document":document}))
            })
            .collect()
    }
}
