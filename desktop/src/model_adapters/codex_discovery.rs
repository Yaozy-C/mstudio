//! Discover native executables without asking users for installation paths.
use std::path::{Path, PathBuf};

fn add(paths: &mut Vec<PathBuf>, path: PathBuf) {
    if path.is_file() && !paths.contains(&path) {
        paths.push(path);
    }
}

fn npm_binaries(paths: &mut Vec<PathBuf>, root: &Path) {
    for arch in ["x86_64", "aarch64"] {
        let package_arch = if arch == "x86_64" { "x64" } else { "arm64" };
        let exe = format!("vendor/{arch}-pc-windows-msvc/codex/codex.exe");
        for package in [
            root.join("@openai/codex"),
            root.join(format!("@openai/codex-win32-{package_arch}")),
            root.join(format!(
                "@openai/codex/node_modules/@openai/codex-win32-{package_arch}"
            )),
        ] {
            add(paths, package.join(&exe));
        }
    }
}

pub async fn binaries() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    if let Some(path) = std::env::var_os("MSTUDIO_CODEX_BIN") {
        add(&mut paths, path.into());
    }
    #[cfg(target_os = "macos")]
    {
        let mut roots = vec![PathBuf::from("/Applications")];
        if let Some(home) = std::env::var_os("HOME") {
            roots.push(PathBuf::from(home).join("Applications"));
        }
        for root in roots {
            for app in ["ChatGPT.app", "Codex.app"] {
                let resources = root.join(app).join("Contents/Resources");
                for exe in ["codex-cli/CodexCLI.app/Contents/MacOS/codex", "codex"] {
                    add(&mut paths, resources.join(exe));
                }
            }
        }
        for exe in ["/opt/homebrew/bin/codex", "/usr/local/bin/codex"] {
            add(&mut paths, exe.into());
        }
    }
    if let Some(path) = std::env::var_os("PATH") {
        for dir in std::env::split_paths(&path) {
            add(
                &mut paths,
                dir.join(if cfg!(windows) { "codex.exe" } else { "codex" }),
            );
            npm_binaries(&mut paths, &dir.join("node_modules"));
        }
    }
    for key in ["APPDATA", "NPM_CONFIG_PREFIX"] {
        if let Some(dir) = std::env::var_os(key) {
            let dir = PathBuf::from(dir);
            npm_binaries(&mut paths, &dir.join("node_modules"));
            npm_binaries(&mut paths, &dir.join("npm/node_modules"));
        }
    }
    if let Some(dir) = std::env::var_os("LOCALAPPDATA") {
        add(
            &mut paths,
            PathBuf::from(dir).join("Programs/OpenAI/Codex/bin/codex.exe"),
        );
    }
    if let Some(home) = std::env::var_os("HOME") {
        add(&mut paths, PathBuf::from(home).join(".local/bin/codex"));
    }
    #[cfg(windows)]
    windows_apps(&mut paths).await;
    paths
}

#[cfg(windows)]
async fn windows_apps(paths: &mut Vec<PathBuf>) {
    // Query package metadata only; no profile scripts or user-supplied shell text.
    let mut command = tokio::process::Command::new("powershell.exe");
    command.creation_flags(0x08000000);
    command.args(["-NoProfile", "-NonInteractive", "-Command",
        "Get-AppxPackage | Where-Object { $_.Name -match 'OpenAI|ChatGPT|Codex' } | Select-Object -ExpandProperty InstallLocation"])
        .kill_on_drop(true);
    if let Ok(Ok(output)) =
        tokio::time::timeout(std::time::Duration::from_secs(4), command.output()).await
        && output.status.success()
    {
        for root in String::from_utf8_lossy(&output.stdout).lines().take(16) {
            scan_app(paths, Path::new(root.trim()), 5);
        }
    }
}

#[cfg(windows)]
fn scan_app(paths: &mut Vec<PathBuf>, root: &Path, depth: usize) {
    if depth == 0 {
        return;
    }
    let Ok(entries) = std::fs::read_dir(root) else {
        return;
    };
    for entry in entries.flatten().take(128) {
        let Ok(kind) = entry.file_type() else {
            continue;
        };
        if kind.is_file()
            && entry
                .file_name()
                .to_string_lossy()
                .eq_ignore_ascii_case("codex.exe")
        {
            add(paths, entry.path());
        } else if kind.is_dir() {
            scan_app(paths, &entry.path(), depth - 1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovers_nested_npm_packages_for_both_architectures_without_duplicates() {
        let root = std::env::temp_dir().join(format!("codex-paths-{}", mstudio::media::id()));
        let exe = root.join("@openai/codex/node_modules/@openai/codex-win32-arm64/vendor/aarch64-pc-windows-msvc/codex/codex.exe");
        std::fs::create_dir_all(exe.parent().unwrap()).unwrap();
        std::fs::write(&exe, "fixture").unwrap();
        let mut paths = Vec::new();
        npm_binaries(&mut paths, &root);
        npm_binaries(&mut paths, &root);
        assert_eq!(paths, vec![exe]);
        std::fs::remove_dir_all(root).unwrap();
    }
}
