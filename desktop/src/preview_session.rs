//! Session identity and ownership share one short lock; never wait while holding it.
use std::sync::Arc;
pub struct Session<T> {
    requested: String,
    active: Option<Arc<T>>,
}
impl<T> Session<T> {
    pub const fn new() -> Self {
        Self {
            requested: String::new(),
            active: None,
        }
    }
    pub fn begin(&mut self, token: String) -> Option<Arc<T>> {
        self.requested = token;
        self.active.take()
    }
    pub fn current(&self, token: &str) -> bool {
        self.requested == token
    }
    pub fn install(&mut self, token: &str, value: Arc<T>) -> Result<(), Arc<T>> {
        if !self.current(token) {
            return Err(value);
        }
        self.active = Some(value);
        Ok(())
    }
    pub fn get(&self, token: &str) -> Option<Arc<T>> {
        self.current(token).then(|| self.active.clone()).flatten()
    }
    pub fn close(&mut self, token: &str) -> Option<Arc<T>> {
        if !self.current(token) {
            return None;
        }
        self.requested.clear();
        self.active.take()
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stale_open_and_close_cannot_replace_or_close_new_session() {
        let mut session = Session::new();
        session.begin("a".into());
        session.begin("b".into());
        assert!(session.install("a", Arc::new(1)).is_err());
        session.install("b", Arc::new(2)).unwrap();
        assert!(session.close("a").is_none());
        assert_eq!(*session.get("b").unwrap(), 2);
        let handle = session.get("b").unwrap();
        assert_eq!(*session.close("b").unwrap(), 2);
        assert!(session.get("b").is_none());
        assert_eq!(*handle, 2);
    }
    #[test]
    fn closing_during_preparation_prevents_late_install() {
        let mut session = Session::new();
        session.begin("a".into());
        session.close("a");
        assert!(session.install("a", Arc::new(1)).is_err());
    }
}
