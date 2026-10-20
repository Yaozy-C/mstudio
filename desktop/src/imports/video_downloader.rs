//! Official standalone yt-dlp, cached outside the application bundle.
use anyhow::{Context, Result, ensure};
use sha2::{Digest, Sha256};
use std::path::{Path, PathBuf};
use tokio::sync::Mutex;
static INSTALL: Mutex<()> = Mutex::const_new(());

pub async fn executable(cache: &Path) -> Result<PathBuf> {
    let _lock = INSTALL.lock().await;
    let name = if cfg!(target_os = "macos") {
        "yt-dlp_macos"
    } else if cfg!(windows) {
        "yt-dlp.exe"
    } else if cfg!(target_arch = "aarch64") {
        "yt-dlp_linux_aarch64"
    } else {
        "yt-dlp_linux"
    };
    let target = cache.join(name);
    if target.is_file() {
        return Ok(target);
    }
    std::fs::create_dir_all(cache)?;
    let client = reqwest::Client::builder()
        .user_agent("Mstudio video import")
        .timeout(std::time::Duration::from_secs(300))
        .build()?;
    let release: serde_json::Value = client
        .get("https://api.github.com/repos/yt-dlp/yt-dlp/releases/latest")
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;
    let tag = release["tag_name"]
        .as_str()
        .context("无法获取视频下载器版本")?;
    ensure!(
        tag.chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '.' || c == '-'),
        "下载器版本无效"
    );
    let base = format!("https://github.com/yt-dlp/yt-dlp/releases/download/{tag}");
    let checksums = client
        .get(format!("{base}/SHA2-256SUMS"))
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let expected = checksum(&checksums, name).context("视频下载器校验信息缺失")?;
    let mut response = client
        .get(format!("{base}/{name}"))
        .send()
        .await?
        .error_for_status()?;
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            bytes.len() + chunk.len() <= 100 * 1024 * 1024,
            "视频下载器文件过大"
        );
        bytes.extend_from_slice(&chunk);
    }
    ensure!(
        format!("{:x}", Sha256::digest(&bytes)) == expected,
        "视频下载器校验失败，请重试"
    );
    let temp = cache.join(format!("{}.part", mstudio::media::id()));
    let result = (|| -> Result<()> {
        std::fs::write(&temp, bytes)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&temp, std::fs::Permissions::from_mode(0o755))?;
        }
        std::fs::rename(&temp, &target)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result?;
    Ok(target)
}
fn checksum<'a>(text: &'a str, name: &str) -> Option<&'a str> {
    text.lines().find_map(|line| {
        let mut fields = line.split_whitespace();
        let hash = fields.next()?;
        (fields.next()?.trim_start_matches('*') == name && hash.len() == 64).then_some(hash)
    })
}

#[cfg(test)]
pub fn command(executable: &Path, work: &Path, url: &str) -> tokio::process::Command {
    build_command(executable, work, url, false)
}
pub(super) fn progress_command(
    executable: &Path,
    work: &Path,
    url: &str,
) -> tokio::process::Command {
    build_command(executable, work, url, true)
}
fn build_command(
    executable: &Path,
    work: &Path,
    url: &str,
    progress: bool,
) -> tokio::process::Command {
    let mut command = tokio::process::Command::new(executable);
    command
        .kill_on_drop(true)
        .args([
            "--ignore-config",
            "--no-plugin-dirs",
            "--no-remote-components",
            "--no-playlist",
            "--playlist-items",
            "1",
            "--no-progress",
            "--no-warnings",
            "--no-colors",
            "--socket-timeout",
            "30",
            "--retries",
            "2",
            "--fragment-retries",
            "2",
            "--max-filesize",
            "1G",
            "--match-filters",
            "!is_live",
            "--format",
            "bv*+ba/b",
            "--merge-output-format",
            "mp4/mkv",
            "--write-info-json",
            "--no-write-playlist-metafiles",
        ])
        .arg("--ffmpeg-location")
        .arg(mstudio::media::binary("ffmpeg"))
        .arg("--output")
        .arg(work.join("source.%(ext)s"));
    // Finder launches do not inherit Homebrew's PATH. Enable an existing JS runtime explicitly.
    for (runtime, prefixes) in [
        ("deno", ["/opt/homebrew/bin/deno", "/usr/local/bin/deno"]),
        ("node", ["/opt/homebrew/bin/node", "/usr/local/bin/node"]),
        ("bun", ["/opt/homebrew/bin/bun", "/usr/local/bin/bun"]),
    ] {
        if let Some(path) = prefixes.iter().find(|p| Path::new(p).is_file()) {
            command
                .arg("--js-runtimes")
                .arg(format!("{runtime}:{path}"));
        }
    }
    if progress {
        command.args([
            "--progress",
            "--newline",
            "--progress-delta",
            "0.3",
            "--progress-template",
            "download:MSTUDIO_PROGRESS %(progress)j",
        ]);
    }
    command.arg("--").arg(url);
    #[cfg(windows)]
    command.creation_flags(0x08000000);
    command
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn uses_exact_official_asset_checksum() {
        let hash = "a".repeat(64);
        let list = format!("{hash}  yt-dlp_macos\n{}  yt-dlp.exe", "b".repeat(64));
        assert_eq!(checksum(&list, "yt-dlp_macos"), Some(hash.as_str()));
        assert!(checksum(&list, "yt-dlp").is_none());
    }
    #[test]
    fn downloads_one_original_with_no_user_configuration_or_shell() {
        let command = command(
            Path::new("/tmp/yt-dlp"),
            Path::new("/tmp/a b"),
            "https://example.com/watch?v=x&list=y",
        );
        let args = command
            .as_std()
            .get_args()
            .map(|a| a.to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        assert!(args.contains(&"--ignore-config".into()));
        assert!(args.contains(&"--no-plugin-dirs".into()));
        assert!(args.contains(&"--no-playlist".into()));
        assert_eq!(args.last().unwrap(), "https://example.com/watch?v=x&list=y");
        assert_eq!(args[args.len() - 2], "--");
    }
}
