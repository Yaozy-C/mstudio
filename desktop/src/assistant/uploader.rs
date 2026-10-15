//! Shared content-addressed uploads, with platform-specific capabilities and projections.
//! Remote IDs exist only in request projections; journal pixels stay reopenable.
#![allow(clippy::result_large_err)] // Rig owns the HTTP error type at this transport boundary.
use crate::database::Store;
use rig_core::http_client::{Error, Result};
use rusqlite::Connection;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    sync::{Arc, Mutex, OnceLock, Weak},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

pub mod backends;
mod source;
use backends::Backend;
pub use source::Source;
mod index;
mod projection;
mod transport;
mod upload;
pub use index::Uploaded;
pub use transport::FileTransport;
#[cfg(test)]
mod live_tests;
#[cfg(test)]
mod openai_tests;
#[cfg(test)]
mod sdk_tests;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod transport_tests;

const LIFETIME: i64 = 7 * 24 * 60 * 60;
const REFRESH_MARGIN: i64 = 60 * 60;

pub struct Uploader {
    backend: Arc<dyn Backend>,
    root: String,
    key: String,
    scope: String,
    db: Mutex<Connection>,
    client: reqwest::Client,
}

#[derive(Clone)]
struct Used {
    variant: String,
    id: String,
}

fn now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_secs() as i64
}

fn error(message: &str) -> Error {
    Error::instance(std::io::Error::other(message.to_owned()))
}

impl Uploader {
    pub fn new(
        backend: Arc<dyn Backend>,
        root: &str,
        key: &str,
        store: Option<&Store>,
    ) -> Result<Arc<Self>> {
        let db = if let Some(store) = store {
            let path: String = store
                .db
                .lock()
                .unwrap()
                .query_row(
                    "SELECT file FROM pragma_database_list WHERE name='main'",
                    [],
                    |r| r.get(0),
                )
                .map_err(Error::instance)?;
            Connection::open(path)
        } else {
            Connection::open_in_memory()
        }
        .map_err(Error::instance)?;
        db.busy_timeout(Duration::from_secs(5))
            .map_err(Error::instance)?;
        index::init(&db)?;
        let root = root.trim_end_matches('/').to_owned();
        let scope = format!(
            "{:x}",
            Sha256::digest(format!("{}\0{root}\0{key}", backend.name()))
        );
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(20))
            .timeout(Duration::from_secs(60))
            .build()
            .map_err(Error::instance)?;
        Ok(Arc::new(Self {
            backend,
            root,
            key: key.into(),
            scope,
            db: Mutex::new(db),
            client,
        }))
    }

    pub async fn resolve(&self, image: &Source) -> Result<String> {
        self.backend.validate(image)?;
        type Locks = Mutex<HashMap<String, Weak<tokio::sync::Mutex<()>>>>;
        static LOCKS: OnceLock<Locks> = OnceLock::new();
        let lock = {
            let mut locks = LOCKS.get_or_init(Default::default).lock().unwrap();
            locks.retain(|_, lock| lock.strong_count() > 0);
            let key = format!("{}:{}", self.scope, image.variant);
            let lock = locks.get(&key).and_then(Weak::upgrade).unwrap_or_default();
            locks.insert(key, Arc::downgrade(&lock));
            lock
        };
        // Recheck durable cache under the shared lock: parallel agents upload once.
        let _guard = lock.lock().await;
        if let Some(id) = index::get(&self.db.lock().unwrap(), &self.scope, &image.variant, now())?
        {
            return Ok(id);
        }
        let record = self
            .backend
            .upload(
                upload::UploadRequest {
                    client: &self.client,
                    root: &self.root,
                    key: &self.key,
                },
                image,
            )
            .await?;
        index::put(
            &self.db.lock().unwrap(),
            &self.scope,
            &image.variant,
            &record,
        )?;
        Ok(record.id)
    }

    async fn prepare(&self, original: &[u8]) -> Result<(Vec<u8>, Vec<Used>)> {
        let mut body: Value = serde_json::from_slice(original).map_err(Error::instance)?;
        let images = projection::collect(&body)?;
        let mut used = Vec::new();
        let mut ids: HashMap<String, String> = HashMap::new();
        for (pointer, image) in images {
            let id = match ids.get(&image.variant) {
                Some(id) => id.clone(),
                None => self.resolve(&image).await?,
            };
            ids.insert(image.variant.clone(), id.clone());
            let part = body
                .pointer_mut(&pointer)
                .ok_or_else(|| error("图片请求位置无效"))?;
            self.backend.project(part, &id)?;
            used.push(Used {
                variant: image.variant,
                id,
            });
        }
        Ok((serde_json::to_vec(&body).map_err(Error::instance)?, used))
    }

    fn invalidate_stale(&self, failure: &Error, used: &[Used]) -> Result<bool> {
        let Error::InvalidStatusCodeWithDetails { status, body, .. } = failure else {
            return Ok(false);
        };
        if ![400, 404, 422].contains(&status.as_u16()) || used.is_empty() {
            return Ok(false);
        }
        let value: Value = serde_json::from_str(body).unwrap_or(Value::Null);
        let detail = ["message", "type", "code"]
            .into_iter()
            .filter_map(|key| value["error"][key].as_str())
            .collect::<Vec<_>>()
            .join(" ");
        let lower = detail.to_ascii_lowercase();
        let names_id = [
            "file_id",
            "file id",
            self.backend.id_prefix(),
            "file not",
            "file expired",
            "file deleted",
        ]
        .iter()
        .any(|name| lower.contains(name));
        if !names_id
            || ![
                "expired",
                "not found",
                "not_found",
                "deleted",
                "does not exist",
                "invalid",
                "not created under",
            ]
            .iter()
            .any(|reason| lower.contains(reason))
        {
            return Ok(false);
        }
        let exact: Vec<_> = used
            .iter()
            .filter(|file| {
                detail
                    .split(|c: char| !c.is_ascii_alphanumeric() && c != '-' && c != '_')
                    .any(|word| word == file.id)
            })
            .collect();
        let selected = if exact.is_empty() {
            used.iter().collect()
        } else {
            exact
        };
        let db = self.db.lock().unwrap();
        for file in selected {
            index::remove(&db, &self.scope, &file.variant, &file.id)?;
        }
        Ok(true)
    }
}
