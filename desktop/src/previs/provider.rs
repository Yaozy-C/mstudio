//! One plain completion. Intentionally independent of the project Agent/harness.
use crate::{model_adapters::codex_rpc::Rpc, models::Model};
use anyhow::{Context, Result, bail, ensure};
use serde_json::{Value, json};
use std::time::Duration;
use tokio_util::sync::CancellationToken;

#[path = "provider_response.rs"]
mod response;

pub async fn complete(
    model: &Model,
    key: &str,
    system: &str,
    context: &str,
    token: &CancellationToken,
) -> Result<String> {
    let request = async {
        if model.profile.adapter == "codex" {
            return codex(&model.profile.model, system, context).await;
        }
        let headers = crate::models::transport::headers(&model.profile.endpoint, key)?;
        let mut client = reqwest::Client::builder()
            .default_headers(headers)
            .redirect(reqwest::redirect::Policy::none())
            .connect_timeout(Duration::from_secs(30))
            .timeout(Duration::from_secs(900));
        if model.profile.adapter == "openai-compatible" {
            // Active streams may run for minutes; only silence is a read timeout.
            client = client.read_timeout(Duration::from_secs(120));
        }
        let client = client.build()?;
        let base = model.profile.endpoint.trim_end_matches('/');
        let (url, body) = match model.profile.adapter.as_str() {
            "openai-compatible" => (
                format!("{base}/chat/completions"),
                json!({"model":model.profile.model,"stream":true,"messages":[{"role":"system","content":system},{"role":"user","content":context}]}),
            ),
            "openai-responses" => (
                format!("{base}/responses"),
                json!({"model":model.profile.model,"instructions":system,"input":context}),
            ),
            "anthropic-native" => (
                format!("{base}/messages"),
                json!({"model":model.profile.model,"system":system,"messages":[{"role":"user","content":context}],"max_tokens":16000}),
            ),
            "gemini-native" => (
                format!(
                    "{base}/v1beta/models/{}:generateContent",
                    model.profile.model
                ),
                json!({"systemInstruction":{"parts":[{"text":system}]},"contents":[{"role":"user","parts":[{"text":context}]}]}),
            ),
            _ => bail!("此连接不支持预演 Chat"),
        };
        let mut req = client.post(url).json(&body);
        req = match model.profile.adapter.as_str() {
            "anthropic-native" => req
                .header("x-api-key", key)
                .header("anthropic-version", "2023-06-01"),
            "gemini-native" => req.header("x-goog-api-key", key),
            _ => req.bearer_auth(if key.is_empty() { "local" } else { key }),
        };
        let response = req.send().await.context("预演 Chat 连接失败")?;
        response::read(response, &model.profile.adapter).await
    };
    tokio::select! {
        r = tokio::time::timeout(Duration::from_secs(900),request) => r.map_err(|_| {
            crate::app_error::AppError::new("NETWORK_ERROR", "previs_chat", "预演 Chat 超时，文件未修改")
        })?,
        _ = token.cancelled() => Err(crate::app_error::AppError::new("CHAT_STOPPED", "previs_chat", "预演 Chat 已停止，文件未修改").into()),
    }
}
pub fn text(adapter: &str, v: &Value) -> Result<String> {
    let result = match adapter {
        "openai-compatible" => {
            ensure!(
                v["choices"][0]["finish_reason"] != "length",
                "模型输出被截断，请缩小本次修改范围"
            );
            ensure!(
                v["choices"][0]["message"]["tool_calls"]
                    .as_array()
                    .is_none_or(|a| a.is_empty()),
                "预演 Chat 不执行工具调用"
            );
            v["choices"][0]["message"]["content"]
                .as_str()
                .unwrap_or("")
                .to_owned()
        }
        "openai-responses" => {
            ensure!(
                v["status"] != "incomplete",
                "模型输出被截断，请缩小本次修改范围"
            );
            v["output"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|m| m["type"] == "message")
                .flat_map(|m| m["content"].as_array().into_iter().flatten())
                .filter_map(|p| p["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        }
        "anthropic-native" => {
            ensure!(
                v["stop_reason"] != "max_tokens",
                "模型输出被截断，请缩小本次修改范围"
            );
            ensure!(
                !v["content"]
                    .as_array()
                    .into_iter()
                    .flatten()
                    .any(|p| p["type"] == "tool_use"),
                "预演 Chat 不执行工具调用"
            );
            v["content"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|p| p["type"] == "text")
                .filter_map(|p| p["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        }
        "gemini-native" => {
            ensure!(
                v["candidates"][0]["finishReason"] != "MAX_TOKENS",
                "模型输出被截断，请缩小本次修改范围"
            );
            v["candidates"][0]["content"]["parts"]
                .as_array()
                .into_iter()
                .flatten()
                .filter(|p| p["thought"] != true)
                .filter_map(|p| p["text"].as_str())
                .collect::<Vec<_>>()
                .join("\n")
        }
        _ => bail!("未知预演 Chat 协议"),
    };
    ensure!(!result.trim().is_empty(), "模型没有返回预演修改");
    Ok(result)
}
async fn codex(model: &str, system: &str, context: &str) -> Result<String> {
    let mut rpc = Rpc::start_text().await?;
    let account = rpc
        .call("account/read", json!({"refreshToken":false}))
        .await?;
    ensure!(
        account["account"]["type"] == "chatgpt",
        "请先在服务连接中登录 Codex"
    );
    let result = async {
        let started = rpc.call("thread/start",json!({"model":model,"ephemeral":true,"cwd":std::env::temp_dir(),"approvalPolicy":"untrusted","sandbox":"read-only","dynamicTools":[],"developerInstructions":format!("You are a plain text code editor. Do not use shell, filesystem, web, MCP, plugins or any tool. Return only the requested JSON. {system}"),"config":{"features":{"shell_tool":false,"apply_patch_freeform":false},"web_search":"disabled"}})).await?;
        let thread = started["thread"]["id"].as_str().context("Codex 未返回会话")?.to_owned();
        let started = rpc.call("turn/start",json!({"threadId":thread,"input":[{"type":"text","text":context}]})).await?;
        let turn = started["turn"]["id"].as_str().context("Codex 未返回回合")?.to_owned();
        let mut output = String::new();
        loop {
            let event = rpc.next().await?;
            let p = &event["params"];
            if p["threadId"].as_str().is_some_and(|s|s!=thread) || p["turnId"].as_str().is_some_and(|s|s!=turn) {continue;}
            match event["method"].as_str() {
                Some("item/agentMessage/delta") => {output.push_str(p["delta"].as_str().unwrap_or(""));ensure!(output.len()<=2_000_000,"模型回复过长");},
                Some("item/completed") if p["item"]["type"]=="agentMessage" => { if let Some(text)=p["item"]["text"].as_str() {output=text.to_owned();} },
                Some("turn/completed") => {ensure!(p["turn"]["status"]=="completed","Codex 请求未完成");ensure!(!output.is_empty(),"Codex 没有返回修改");return Ok(output);},
                Some("error") => bail!("Codex 预演 Chat 请求失败"),
                _ if event.get("id").is_some() && event.get("method").is_some() => {
                    rpc.send(json!({"id":event["id"],"error":{"code":-32601,"message":"Plain previs chat has no tools"}})).await?;
                    bail!("预演 Chat 不执行工具调用");
                },
                _ => {},
            }
        }
    }.await;
    rpc.stop().await;
    result
}
