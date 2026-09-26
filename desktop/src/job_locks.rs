use std::collections::HashMap;
use std::sync::{Arc, LazyLock, Mutex, Weak};
use tokio::sync::{Mutex as AsyncMutex, OwnedMutexGuard};
// Serialize refresh/cancel/recovery/import for one job without blocking other jobs.
type LockMap = HashMap<String, Weak<AsyncMutex<()>>>;
static LOCKS: LazyLock<Mutex<LockMap>> = LazyLock::new(Mutex::default);
pub async fn acquire(id: &str) -> OwnedMutexGuard<()> {
    let lock = {
        let mut locks = LOCKS.lock().unwrap();
        locks.retain(|_, lock| lock.strong_count() > 0);
        let lock = locks.get(id).and_then(Weak::upgrade).unwrap_or_default();
        locks.insert(id.into(), Arc::downgrade(&lock));
        lock
    };
    lock.lock_owned().await
}
