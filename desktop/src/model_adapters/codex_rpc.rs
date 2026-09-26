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
        let binary = std::env::var_os("MSTUDIO_CODEX_BIN").unwrap_or_else(|| {
            let bundled = "/Applications/ChatGPT.app/Contents/Resources/codex";
            if std::path::Path::new(bundled).is_file() {
                bundled.into()
            } else {
                "codex".into()
            }
        });
        let mut child = tokio::process::Command::new(binary)
            .args(["app-server", "--stdio", "--enable", "image_generation"])
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .kill_on_drop(true)
            .spawn()
            .context("无法启动本机 Codex，请安装 Codex 或设置 MSTUDIO_CODEX_BIN")?;
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
