use super::Store;
use rusqlite::Connection;
use std::{
    ops::{Deref, DerefMut},
    sync::MutexGuard,
    time::Instant,
};
pub struct Access<'a> {
    connection: MutexGuard<'a, Connection>,
    held: Instant,
}
impl Store {
    pub fn conn(&self) -> anyhow::Result<Access<'_>> {
        let start = Instant::now();
        let connection = self.db.lock().map_err(|_| {
            anyhow::Error::from(crate::app_error::AppError::new(
                "STORAGE_FAILED",
                "database_lock",
                "Database access interrupted; restart the application",
            ))
        })?;
        let wait_ms = start.elapsed().as_millis() as u64;
        if wait_ms >= 50 {
            tracing::warn!(event = "database_lock_wait", duration_ms = wait_ms);
        }
        Ok(Access {
            connection,
            held: Instant::now(),
        })
    }
}
impl Deref for Access<'_> {
    type Target = Connection;
    fn deref(&self) -> &Connection {
        &self.connection
    }
}
impl DerefMut for Access<'_> {
    fn deref_mut(&mut self) -> &mut Connection {
        &mut self.connection
    }
}
impl Drop for Access<'_> {
    fn drop(&mut self) {
        let duration_ms = self.held.elapsed().as_millis() as u64;
        if duration_ms >= 50 {
            tracing::warn!(event = "database_lock_held", duration_ms);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn poisoned_connection_returns_a_coded_error_instead_of_panicking() {
        let root = std::env::temp_dir().join(format!("mstudio-lock-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let _guard = store.db.lock().unwrap();
            panic!("synthetic poison");
        }));
        let error = store.conn().err().unwrap();
        let issue: serde_json::Value = serde_json::from_str(&error.to_string()).unwrap();
        assert_eq!(issue["code"], "STORAGE_FAILED");
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
