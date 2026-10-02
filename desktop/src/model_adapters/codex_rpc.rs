//! Local Codex JSON-RPC transport. Authentication remains owned by Codex.
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout},
};

fn resolve_binary(
    override_path: Option<std::ffi::OsString>,
    candidates: &[std::path::PathBuf],
) -> std::path::PathBuf {
    if let Some(path) = override_path {
        return path.into();
    }
    candidates
        .iter()
        .find(|path| path.is_file())
        .cloned()
        .unwrap_or_else(|| "codex".into())
}

fn codex_binary() -> std::path::PathBuf {
    let mut candidates = Vec::new();
    let mut roots = vec![std::path::PathBuf::from("/Applications")];
    if let Some(home) = std::env::var_os("HOME") {
        roots.push(std::path::PathBuf::from(home).join("Applications"));
    }
    for root in roots {
        for bundle in ["ChatGPT.app", "Codex.app"] {
            let resources = root.join(bundle).join("Contents/Resources");
            candidates.push(resources.join("codex-cli/CodexCLI.app/Contents/MacOS/codex"));
            candidates.push(resources.join("codex"));
        }
    }
    candidates
        .extend(["/opt/homebrew/bin/codex", "/usr/local/bin/codex"].map(std::path::PathBuf::from));
    #[cfg(windows)]
    {
        if let Some(path) = std::env::var_os("PATH") {
            candidates.extend(std::env::split_paths(&path).map(|p| p.join("codex.exe")));
        }
        // npm's launcher is a .cmd wrapper; prefer the bundled native executable.
        if let Some(appdata) = std::env::var_os("APPDATA") {
            let npm = std::path::PathBuf::from(appdata).join("npm/node_modules/@openai/codex");
            for relative in [
                "vendor/x86_64-pc-windows-msvc/codex/codex.exe",
                "node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/codex/codex.exe",
            ] {
                candidates.push(npm.join(relative));
            }
        }
    }
    resolve_binary(std::env::var_os("MSTUDIO_CODEX_BIN"), &candidates)
}

pub struct Rpc {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    sequence: u64,
    pending: std::collections::VecDeque<Value>,
}
impl Rpc {
    pub async fn start() -> Result<Self> {
        let binary = codex_binary();
        let mut command = tokio::process::Command::new(&binary);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        let mut child = command
            .args(["app-server", "--stdio", "--enable", "image_generation"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .map_err(|error| {
                anyhow::anyhow!("无法启动本机 Codex（{}）：{}", binary.display(), error)
            })?;
        let input = child.stdin.take().context("Codex stdin 不可用")?;
        let output = BufReader::new(child.stdout.take().context("Codex stdout 不可用")?);
        let mut rpc = Self {
            child,
            input,
            output,
            sequence: 0,
            pending: Default::default(),
        };
        rpc.call("initialize", json!({"clientInfo":{"name":"mstudio","version":"0.1.0"},"capabilities":{"experimentalApi":true}})).await?;
        rpc.send(json!({"method":"initialized","params":{}}))
            .await?;
        Ok(rpc)
    }
    pub async fn send(&mut self, value: Value) -> Result<()> {
        self.input
            .write_all(format!("{value}\n").as_bytes())
            .await?;
        self.input.flush().await?;
        Ok(())
    }
    pub async fn next(&mut self) -> Result<Value> {
        if let Some(event) = self.pending.pop_front() {
            return Ok(event);
        }
        self.read_wire().await
    }
    async fn read_wire(&mut self) -> Result<Value> {
        let mut bytes = Vec::new();
        loop {
            let buffer = self.output.fill_buf().await?;
            ensure!(!buffer.is_empty(), "Codex 进程提前结束");
            let end = buffer.iter().position(|b| *b == b'\n');
            let count = end.map_or(buffer.len(), |i| i + 1);
            ensure!(bytes.len() + count <= 90_000_000, "Codex 响应超过大小限制");
            bytes.extend_from_slice(&buffer[..count]);
            self.output.consume(count);
            if end.is_some() {
                return Ok(serde_json::from_slice(&bytes)?);
            }
        }
    }
    pub async fn call(&mut self, method: &str, params: Value) -> Result<Value> {
        self.sequence += 1;
        let id = self.sequence;
        self.send(json!({"id":id,"method":method,"params":params}))
            .await?;
        loop {
            let event = self.read_wire().await?;
            if event["id"] == id {
                if let Some(error) = event.get("error") {
                    bail!("Codex: {}", error["message"]);
                }
                return Ok(event["result"].clone());
            }
            self.decline(&event).await?;
            if event.get("id").is_none() {
                self.pending.push_back(event);
            }
        }
    }
    pub async fn decline(&mut self, event: &Value) -> Result<()> {
        if event.get("id").is_some() && event.get("method").is_some() {
            self.send(json!({"id":event["id"],"error":{"code":-32601,"message":"Mstudio image provider does not execute interactive or external tool requests"}})).await?;
        }
        Ok(())
    }
    pub async fn stop(&mut self) {
        let _ = self.child.kill().await;
        let _ = self.child.wait().await;
    }
}
#[tauri::command]
pub async fn codex_image_status() -> Result<Value, String> {
    let result = tokio::time::timeout(std::time::Duration::from_secs(20), async {
        let mut rpc = Rpc::start().await?;
        let result: Result<Value> = async {
            let account = rpc.call("account/read", json!({"refreshToken":false})).await?;
            let capabilities = rpc.call("modelProvider/capabilities/read", json!({})).await?;
            Ok(json!({"loggedIn":account["account"]["type"] == "chatgpt", "imageGeneration":capabilities["imageGeneration"],"modelVerified":false}))
        }.await;
        rpc.stop().await;
        result
    }).await;
    result
        .map_err(|_| "连接 Codex 超时".to_string())?
        .map_err(|e| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn discovery_preserves_override_and_skips_missing_app_locations() {
        let root = std::env::temp_dir().join(format!("codex-discovery-{}", mstudio::media::id()));
        std::fs::create_dir_all(&root).unwrap();
        let installed = root.join("codex");
        std::fs::write(&installed, "fixture").unwrap();
        let candidates = [root.join("missing"), installed.clone()];
        assert_eq!(resolve_binary(None, &candidates), installed);
        assert_eq!(
            resolve_binary(Some("custom-codex".into()), &candidates),
            std::path::PathBuf::from("custom-codex")
        );
        assert_eq!(resolve_binary(None, &[]), std::path::PathBuf::from("codex"));
        std::fs::remove_dir_all(root).unwrap();
    }
    #[tokio::test]
    #[ignore = "requires a locally installed Codex; handshake only, no generation"]
    async fn installed_codex_initializes_without_generation() {
        let mut rpc = tokio::time::timeout(std::time::Duration::from_secs(20), Rpc::start())
            .await
            .unwrap()
            .unwrap();
        rpc.stop().await;
    }
}
