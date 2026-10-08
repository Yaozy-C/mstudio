use super::config::Profile;
use rig_agent::{agent::AgentBuilder, prelude::*};
use rig_core::providers::{anthropic, gemini, openai};
use std::time::Duration;

pub(super) fn http_client(timeout: Duration) -> Result<rig_http::Client, String> {
    rig_http::Client::builder()
        .redirect(rig_http::redirect::Policy::none())
        .connect_timeout(Duration::from_secs(20))
        .timeout(timeout)
        .build()
        .map_err(|_| "无法创建 HTTP 客户端".into())
}

pub fn builder(profile: &Profile, key: &str) -> Result<AgentBuilder, String> {
    if profile.adapter == "codex" {
        return Ok(AgentBuilder::new(super::codex_provider::CodexModel::new(
            profile.model.clone(),
        )));
    }
    // Each model request gets its own budget, including after tool execution.
    let http = http_client(Duration::from_secs(240))?;
    let key = if key.is_empty() { "local" } else { key };
    macro_rules! client {
        ($provider:ident) => {
            $provider::Client::builder()
                .api_key(key)
                .base_url(profile.endpoint.trim_end_matches('/'))
                .http_client(http)
                .build()
                .map_err(|_| "无法创建模型客户端")?
        };
    }
    Ok(match profile.adapter.as_str() {
        "gemini-native" => client!(gemini).agent(&profile.model),
        "anthropic-native" => client!(anthropic).agent(&profile.model).max_tokens(8192),
        "openai-responses" => client!(openai)
            .agent(&profile.model)
            .additional_params(serde_json::json!({"store":false})),
        "openai-compatible" => client!(openai).completions_api().agent(&profile.model),
        _ => return Err("模型协议插件未安装".into()),
    })
}

#[cfg(test)]
mod timeout_tests {
    use super::*;

    #[tokio::test]
    async fn stalled_request_times_out_and_reports_preserved_work() {
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.local_addr().unwrap());
        // Keep the listener alive without responding: the request must not hang.
        let error = http_client(Duration::from_millis(50))
            .unwrap()
            .get(url)
            .send()
            .await
            .unwrap_err();
        assert!(error.is_timeout());
        let error = rig_core::completion::CompletionError::HttpError(
            rig_core::http_client::Error::Instance(Box::new(error)),
        )
        .into();
        let message = super::super::failure::describe(&error, "");
        assert!(message.contains("本次模型请求超时"));
        assert!(message.contains("已完成的操作保留"));
        assert!(!message.contains("127.0.0.1"));
    }

    #[tokio::test]
    async fn request_budget_starts_when_sent_not_when_client_is_created() {
        use std::io::{Read, Write};
        let server = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}", server.local_addr().unwrap());
        let client = http_client(Duration::from_millis(200)).unwrap();
        let thread = std::thread::spawn(move || {
            for _ in 0..2 {
                let (mut stream, _) = server.accept().unwrap();
                stream
                    .set_read_timeout(Some(Duration::from_secs(5)))
                    .unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut byte = [0];
                    stream.read_exact(&mut byte).unwrap();
                    request.push(byte[0]);
                    assert!(request.len() < 16_384);
                }
                stream
                    .write_all(
                        b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\nOK",
                    )
                    .unwrap();
            }
        });
        for _ in 0..2 {
            // Simulate work between requests, exceeding the per-request budget.
            tokio::time::sleep(Duration::from_millis(250)).await;
            assert_eq!(
                client.get(&url).send().await.unwrap().text().await.unwrap(),
                "OK"
            );
        }
        thread.join().unwrap();
    }
}
