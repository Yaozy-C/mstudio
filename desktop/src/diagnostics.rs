//! Operational metadata only: never log prompts, paths, provider bodies or keys.
use std::time::Instant;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;

pub fn init(directory: &std::path::Path) -> anyhow::Result<WorkerGuard> {
    let (subscriber, guard) = subscriber(directory)?;
    subscriber
        .try_init()
        .map_err(|_| anyhow::anyhow!("Diagnostic logger initialization failed"))?;
    tracing::info!(
        event = "application_start",
        version = env!("CARGO_PKG_VERSION")
    );
    Ok(guard)
}
fn subscriber(
    directory: &std::path::Path,
) -> anyhow::Result<(impl tracing::Subscriber + Send + Sync, WorkerGuard)> {
    std::fs::create_dir_all(directory)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(directory, std::fs::Permissions::from_mode(0o700))?;
    }
    let file = tracing_appender::rolling::Builder::new()
        .rotation(tracing_appender::rolling::Rotation::HOURLY)
        .filename_prefix("mstudio")
        .filename_suffix("jsonl")
        .max_log_files(24)
        .build(directory)?;
    let (writer, guard) = tracing_appender::non_blocking(file);
    let subscriber = tracing_subscriber::registry().with(
        tracing_subscriber::fmt::layer()
            .with_ansi(false)
            .with_writer(writer)
            .json()
            .with_filter(
                tracing_subscriber::filter::Targets::new()
                    .with_target("mstudio_desktop", tracing::Level::INFO),
            ),
    );
    Ok((subscriber, guard))
}

pub struct Timing {
    operation: &'static str,
    start: Instant,
}
impl Timing {
    pub fn new(operation: &'static str) -> Self {
        Self {
            operation,
            start: Instant::now(),
        }
    }
}
impl Drop for Timing {
    fn drop(&mut self) {
        let millis = self.start.elapsed().as_millis() as u64;
        if millis >= 50 {
            tracing::warn!(
                event = "slow_operation",
                operation = self.operation,
                duration_ms = millis
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn operational_log_is_written_and_excludes_dependency_payloads() {
        let root =
            std::env::temp_dir().join(format!("mstudio-diagnostics-{}", mstudio::media::id()));
        let (subscriber, guard) = subscriber(&root).unwrap();
        tracing::subscriber::with_default(subscriber, || {
            tracing::info!(target: "reqwest", payload = "SYNTHETIC_SECRET_DO_NOT_LOG");
            let _timing = Timing {
                operation: "domain_execute",
                start: Instant::now() - std::time::Duration::from_millis(60),
            };
        });
        drop(guard);
        let output = std::fs::read_dir(&root)
            .unwrap()
            .map(|entry| std::fs::read_to_string(entry.unwrap().path()).unwrap())
            .collect::<String>();
        assert!(output.contains("slow_operation"));
        assert!(output.contains("domain_execute"));
        assert!(!output.contains("SYNTHETIC_SECRET"));
        std::fs::remove_dir_all(root).unwrap();
    }
}
