//! Bounded JSON/SSE transport for a single plain completion, without tool execution.
use anyhow::{Context, Result, bail, ensure};
use serde_json::Value;

const MAX_BYTES: usize = 2_000_000;

pub(super) async fn read(mut response: reqwest::Response, adapter: &str) -> Result<String> {
    if !response.status().is_success() {
        return Err(crate::app_error::http_error(response, "previs_chat")
            .await
            .into());
    }
    let streaming = adapter == "openai-compatible"
        && response
            .headers()
            .get(reqwest::header::CONTENT_TYPE)
            .and_then(|v| v.to_str().ok())
            .is_some_and(|v| v.split(';').next().unwrap_or("").trim() == "text/event-stream");
    let mut bytes = Vec::new();
    let mut stream = CompletionStream::default();
    let mut received = 0;
    while let Some(chunk) = response
        .chunk()
        .await
        .context("预演回复接收中断，文件未修改")?
    {
        received += chunk.len();
        ensure!(received <= MAX_BYTES, "模型回复超过 2 MB");
        bytes.extend_from_slice(&chunk);
        if streaming {
            while let Some(end) = bytes.iter().position(|b| *b == b'\n') {
                let line = bytes.drain(..=end).collect::<Vec<_>>();
                if stream.line(std::str::from_utf8(&line).context("模型响应不是有效 UTF-8")?)?
                {
                    return stream.finish();
                }
            }
        }
    }
    if streaming {
        if !bytes.is_empty() {
            stream.line(std::str::from_utf8(&bytes).context("模型响应不是有效 UTF-8")?)?;
        }
        stream.event()?;
        stream.finish()
    } else {
        // Some compatible services ignore stream:true and return an ordinary JSON body.
        let value = serde_json::from_slice(&bytes).context("模型响应不是有效 JSON")?;
        super::text(adapter, &value)
    }
}

#[derive(Default)]
struct CompletionStream {
    data: String,
    output: String,
    stopped: bool,
}

impl CompletionStream {
    fn line(&mut self, line: &str) -> Result<bool> {
        let line = line.trim_end_matches(['\r', '\n']);
        if line.is_empty() {
            return self.event();
        }
        if let Some(data) = line.strip_prefix("data:") {
            if !self.data.is_empty() {
                self.data.push('\n');
            }
            self.data.push_str(data.strip_prefix(' ').unwrap_or(data));
        }
        Ok(false)
    }

    fn event(&mut self) -> Result<bool> {
        let data = std::mem::take(&mut self.data);
        if data.trim().is_empty() {
            return Ok(false);
        }
        if data.trim() == "[DONE]" {
            return Ok(true);
        }
        let value: Value = serde_json::from_str(&data).context("模型流式响应不是有效 JSON")?;
        ensure!(
            value.get("error").is_none(),
            "模型服务返回流式错误，文件未修改"
        );
        let Some(choice) = value["choices"]
            .as_array()
            .and_then(|choices| choices.first())
        else {
            return Ok(false); // Usage-only event or heartbeat.
        };
        ensure!(
            choice["delta"]["tool_calls"]
                .as_array()
                .is_none_or(|calls| calls.is_empty()),
            "预演 Chat 不执行工具调用"
        );
        // Deliberately omit reasoning_content: only final content edits the file.
        if let Some(text) = choice["delta"]["content"].as_str() {
            self.output.push_str(text);
        }
        if let Some(reason) = choice["finish_reason"].as_str() {
            match reason {
                "stop" => self.stopped = true,
                "length" => bail!("模型输出被截断，请缩小本次修改范围"),
                "tool_calls" | "function_call" => bail!("预演 Chat 不执行工具调用"),
                _ => bail!("模型未完成预演回复，文件未修改"),
            }
        }
        Ok(self.stopped)
    }

    fn finish(self) -> Result<String> {
        ensure!(self.stopped, "预演回复接收中断，文件未修改");
        ensure!(!self.output.trim().is_empty(), "模型没有返回预演修改");
        Ok(self.output)
    }
}
