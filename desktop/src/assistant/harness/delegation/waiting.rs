//! Ending the parent's wait must not drop the child's execution future.
pub(super) async fn until<T>(
    mut handle: tokio::task::JoinHandle<T>,
    deadline: tokio::time::Instant,
) -> Option<Result<T, tokio::task::JoinError>> {
    tokio::select! {
        biased;
        result = &mut handle => Some(result),
        _ = tokio::time::sleep_until(deadline) => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::{sync::oneshot, time::Instant};
    #[tokio::test]
    async fn expired_parent_does_not_stop_child_and_explicit_cancel_does() {
        let parent = tokio_util::sync::CancellationToken::new();
        let child = parent.child_token();
        let (done, observed) = oneshot::channel();
        let handle = tokio::spawn(async move {
            child.cancelled().await;
            let _ = done.send("cancelled");
        });
        assert!(until(handle, Instant::now()).await.is_none());
        assert!(!parent.is_cancelled());
        parent.cancel();
        assert_eq!(observed.await.unwrap(), "cancelled");
    }
    #[tokio::test]
    async fn child_completes_after_parent_stops_waiting() {
        let (release, start) = oneshot::channel();
        let (done, observed) = oneshot::channel();
        let handle = tokio::spawn(async move {
            start.await.unwrap();
            done.send(7).unwrap();
        });
        assert!(until(handle, Instant::now()).await.is_none());
        release.send(()).unwrap();
        assert_eq!(observed.await.unwrap(), 7);
    }
}
