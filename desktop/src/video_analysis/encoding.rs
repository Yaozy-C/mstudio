use super::progress::Reporter;
use anyhow::Result;
use tokio::io::{AsyncBufReadExt, BufReader};

pub(super) async fn wait(
    mut child: tokio::process::Child,
    duration: f64,
    progress: &Reporter,
) -> Result<std::process::Output> {
    let stdout = child.stdout.take().expect("piped encoding progress");
    let mut lines = BufReader::new(stdout).lines();
    let output = child.wait_with_output();
    tokio::pin!(output);
    loop {
        tokio::select! {
            result = &mut output => return Ok(result?),
            line = lines.next_line() => match line? {
                Some(line) => {
                    if let Some(time) = line.strip_prefix("out_time_us=").and_then(|s| s.parse::<u64>().ok()) {
                        progress.send("encoding", Some(time.min((duration * 1_000_000.) as u64)), Some((duration * 1_000_000.) as u64));
                    }
                }
                None => return Ok(output.await?),
            }
        }
    }
}
