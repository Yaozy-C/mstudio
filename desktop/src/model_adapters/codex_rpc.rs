//! Local Codex JSON-RPC transport. Authentication remains owned by Codex.
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::process::Stdio;
use tokio::{
    io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
    process::{Child, ChildStdin, ChildStdout},
};

pub struct Rpc {
    child: Child,
    input: ChildStdin,
    output: BufReader<ChildStdout>,
    sequence: u64,
    pending: std::collections::VecDeque<Value>,
}
impl Rpc {
    pub async fn start() -> Result<Self> {
        Self::start_mode(true).await
    }
    pub async fn start_text() -> Result<Self> {
        Self::start_mode(false).await
    }
    async fn start_mode(images: bool) -> Result<Self> {
        let mut last = anyhow::anyhow!("未找到 Codex");
        for binary in super::codex_discovery::binaries().await {
            for transport in ["--stdio", "--listen"] {
                match tokio::time::timeout(
                    std::time::Duration::from_secs(4),
                    Self::connect(&binary, transport, images),
                )
                .await
                {
                    Ok(Ok(rpc)) => return Ok(rpc),
                    Ok(Err(error)) => last = error,
                    Err(_) => last = anyhow::anyhow!("连接 Codex 超时"),
                }
            }
        }
        Err(last)
    }
    async fn connect(binary: &std::path::Path, transport: &str, images: bool) -> Result<Self> {
        let mut command = tokio::process::Command::new(binary);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        command.args(["app-server", transport]);
        if transport == "--listen" {
            command.arg("stdio://");
        }
        if images {
            command.args(["--enable", "image_generation"]);
        }
        let mut child = command
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
        rpc.call("initialize", json!({"clientInfo":{"name":"mstudio","version":env!("CARGO_PKG_VERSION")},"capabilities":{"experimentalApi":true}})).await?;
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
#[cfg(test)]
mod tests {
    use super::*;
    #[cfg(unix)]
    #[tokio::test]
    async fn supports_listen_transport_when_legacy_flag_is_rejected() {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!("codex-rpc-{}", mstudio::media::id()));
        std::fs::write(
            &path,
            r#"#!/bin/sh
if [ "$2" = "--stdio" ]; then exit 2; fi
if [ "$2" != "--listen" ] || [ "$3" != "stdio://" ]; then exit 3; fi
read -r request
printf '%s\n' '{"id":1,"result":{}}'
read -r initialized
"#,
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        assert!(Rpc::connect(&path, "--stdio", false).await.is_err());
        let mut rpc = Rpc::connect(&path, "--listen", false).await.unwrap();
        rpc.stop().await;
        std::fs::remove_file(path).unwrap();
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
