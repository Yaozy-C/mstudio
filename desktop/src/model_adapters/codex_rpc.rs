//! Local Codex JSON-RPC transport. Authentication remains owned by Codex.
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
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
#[derive(Debug, Deserialize)]
pub struct RpcError {
    pub code: i64,
    pub message: String,
}
impl std::fmt::Display for RpcError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Codex RPC {}: {}", self.code, self.message)
    }
}
impl std::error::Error for RpcError {}
impl Rpc {
    pub async fn start_text() -> Result<Self> {
        let binary = super::codex_discovery::binary()
            .await?
            .context("未找到 Codex")?;
        tracing::info!(event = "codex_rpc_start", transport = "stdio");
        tokio::time::timeout(std::time::Duration::from_secs(4), Self::connect(&binary))
            .await
            .context("连接 Codex 超时（stdio，4 秒）")?
            .with_context(|| format!("连接本机 Codex 失败（{}，stdio）", binary.display()))
    }
    pub(crate) async fn connect(binary: &std::path::Path) -> Result<Self> {
        let mut command = tokio::process::Command::new(binary);
        #[cfg(windows)]
        command.creation_flags(0x08000000);
        command.args(["app-server", "--listen", "stdio://"]);
        // A fresh process must use HTTP immediately, rather than retrying broken
        // WebSockets before falling back. Built-in provider IDs are reserved.
        for config in [
            "model_provider=\"mstudio_openai_http\"",
            "model_providers.mstudio_openai_http.name=\"OpenAI (Mstudio HTTP)\"",
            "model_providers.mstudio_openai_http.requires_openai_auth=true",
            "model_providers.mstudio_openai_http.supports_websockets=false",
            "model_providers.mstudio_openai_http.wire_api=\"responses\"",
        ] {
            command.args(["-c", config]);
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
            if event.get("method").is_none() && event["id"] == id {
                if let Some(error) = event.get("error") {
                    return Err(serde_json::from_value::<RpcError>(error.clone())
                        .context("Codex 返回了无效的 RPC 错误")?
                        .into());
                }
                return Ok(event["result"].clone());
            }
            // A dynamic tool call may arrive before turn/start's response.
            // Server request IDs have their own namespace and can equal our ID.
            if event["method"] == "item/tool/call" {
                self.pending.push_back(event);
                continue;
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
#[cfg(all(test, unix))]
mod tests {
    use super::*;
    #[tokio::test]
    async fn early_dynamic_call_is_not_declined_or_mistaken_for_rpc_response() {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!("codex-early-{}", mstudio::media::id()));
        std::fs::write(&path, r#"#!/bin/sh
read -r request
printf '%s\n' '{"id":1,"result":{}}'
read -r initialized
read -r request
printf '%s\n' '{"id":2,"method":"item/tool/call","params":{"tool":"echo","callId":"c","arguments":{}}}'
printf '%s\n' '{"id":2,"result":{"turn":{"id":"t"}}}'
read -r response
"#).unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut rpc = Rpc::connect(&path).await.unwrap();
        let result = rpc.call("turn/start", json!({})).await.unwrap();
        assert_eq!(result["turn"]["id"], "t");
        let event = rpc.next().await.unwrap();
        assert_eq!(event["method"], "item/tool/call");
        assert_eq!(event["params"]["callId"], "c");
        rpc.stop().await;
        std::fs::remove_file(path).unwrap();
    }
    #[tokio::test]
    async fn uses_one_explicit_stdio_transport() {
        use std::os::unix::fs::PermissionsExt;
        let path = std::env::temp_dir().join(format!("codex-rpc-{}", mstudio::media::id()));
        std::fs::write(
            &path,
            r#"#!/bin/sh
if [ "$2" != "--listen" ] || [ "$3" != "stdio://" ]; then exit 3; fi
read -r request
printf '%s\n' '{"id":1,"result":{}}'
read -r initialized
"#,
        )
        .unwrap();
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o700)).unwrap();
        let mut rpc = Rpc::connect(&path).await.unwrap();
        rpc.stop().await;
        std::fs::remove_file(path).unwrap();
    }
}
