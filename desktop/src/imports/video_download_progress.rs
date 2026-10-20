//! Drain both pipes while forwarding only structured byte counters, never URLs or logs.
use anyhow::Result;
use tokio::io::{AsyncBufReadExt, BufReader};

pub(super) async fn run(
    mut command: tokio::process::Command,
    progress: &(dyn Fn(u64, Option<u64>, bool) + Send + Sync),
) -> Result<std::process::Output> {
    let mut child = command
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()?;
    let stdout = child.stdout.take().expect("piped download progress");
    let mut lines = BufReader::new(stdout).lines();
    let output = child.wait_with_output();
    tokio::pin!(output);
    loop {
        tokio::select! {
            result = &mut output => return Ok(result?),
            line = lines.next_line() => match line? {
                Some(line) => {
                    if let Some((done, total, fragments)) = counters(&line) { progress(done, total, fragments); }
                }
                None => return Ok(output.await?),
            }
        }
    }
}
fn counters(line: &str) -> Option<(u64, Option<u64>, bool)> {
    let value: serde_json::Value =
        serde_json::from_str(line.strip_prefix("MSTUDIO_PROGRESS ")?).ok()?;
    let total = value["total_bytes"].as_u64().filter(|n| *n > 0);
    if total.is_none()
        && let (Some(done), Some(total)) = (
            value["fragment_index"].as_u64(),
            value["fragment_count"].as_u64().filter(|n| *n > 0),
        )
    {
        return Some((done, Some(total), true));
    }
    Some((value["downloaded_bytes"].as_u64()?, total, false))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn only_exact_known_totals_support_a_percentage() {
        assert_eq!(
            counters("MSTUDIO_PROGRESS {\"downloaded_bytes\":25,\"total_bytes\":100}"),
            Some((25, Some(100), false))
        );
        assert_eq!(
            counters("MSTUDIO_PROGRESS {\"downloaded_bytes\":25,\"total_bytes\":null}"),
            Some((25, None, false))
        );
        assert_eq!(
            counters("MSTUDIO_PROGRESS {\"downloaded_bytes\":25,\"total_bytes\":0}"),
            Some((25, None, false))
        );
        assert_eq!(
            counters(
                r#"MSTUDIO_PROGRESS {"downloaded_bytes":25,"total_bytes":null,"fragment_index":2,"fragment_count":8}"#
            ),
            Some((2, Some(8), true))
        );
        assert!(counters("[download] other log").is_none());
        assert!(counters("MSTUDIO_PROGRESS invalid").is_none());
    }
}
