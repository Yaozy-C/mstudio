use super::config::Profile;
use rig_core::providers::{anthropic, gemini, openai};
use std::time::Duration;

#[cfg(test)]
pub(super) fn http_client(timeout: Duration) -> Result<rig_http::Client, String> {
    configured_client(timeout, rig_http::header::HeaderMap::new())
}
fn configured_client(
    timeout: Duration,
    headers: rig_http::header::HeaderMap,
) -> Result<rig_http::Client, String> {
    rig_http::Client::builder()
        .redirect(rig_http::redirect::Policy::none())
        .default_headers(headers)
        .connect_timeout(Duration::from_secs(20))
        .timeout(timeout)
        .build()
        .map_err(|_| "无法创建 HTTP 客户端".into())
}

pub enum ProviderModel {
    Http(rig_core::DynModel<rig_core::operation::Completion>),
    Codex(super::codex_provider::CodexModel),
}
impl super::model_io::CompletionModel for ProviderModel {
    async fn completion(
        &self,
        request: rig_core::completion::CompletionRequest,
    ) -> Result<rig_core::completion::CompletionResponse, rig_core::ProviderError> {
        match self {
            Self::Http(model) => model.completion(request).await,
            Self::Codex(model) => model.completion(request).await,
        }
    }
    async fn stream(
        &self,
        request: rig_core::completion::CompletionRequest,
    ) -> Result<super::model_io::ModelStream, rig_core::ProviderError> {
        match self {
            Self::Http(model) => super::model_io::CompletionModel::stream(model, request).await,
            Self::Codex(model) => model.stream(request).await,
        }
    }
}
pub fn model(
    profile: &Profile,
    key: &str,
    store: Option<&crate::database::Store>,
) -> Result<ProviderModel, String> {
    if profile.adapter == "codex" {
        return Ok(ProviderModel::Codex(
            super::codex_provider::CodexModel::new(profile.model.clone()),
        ));
    }
    let headers = crate::models::transport::headers(&profile.endpoint, key)
        .map_err(|_| "服务认证配置无效")?;
    let transport =
        rig_reqwest::ReqwestClient::from(configured_client(Duration::from_secs(240), headers)?);
    let http = if let Some(backend) = super::uploader::backends::select(profile) {
        let uploader = super::uploader::Uploader::new(backend.clone(), backend.root(), key, store)
            .map_err(|_| "无法初始化文件上传缓存")?;
        rig_core::http_client::DynHttpClient::new(super::uploader::FileTransport::new(
            transport, uploader,
        ))
    } else {
        rig_core::http_client::DynHttpClient::new(transport)
    };
    let key = if key.is_empty() { "local" } else { key };
    let endpoint = profile.endpoint.trim_end_matches('/');
    let model = match profile.adapter.as_str() {
        "gemini-native" => gemini::GeminiConfig::new(key)
            .with_base_url(endpoint)
            .connect(http)
            .completion(&profile.model)
            .erase(),
        "anthropic-native" => anthropic::AnthropicConfig::new(key)
            .with_base_url(endpoint)
            .connect(http)
            .completion(&profile.model)
            .erase(),
        "openai-responses" => openai::OpenAIConfig::new(key)
            .with_base_url(endpoint)
            .connect(http)
            .responses(&profile.model)
            .erase(),
        "openai-compatible" => openai::OpenAIConfig::with_key(
            &openai::wire::Dialect::gateway("mstudio-compatible", "", ""),
            key,
        )
        .with_base_url(endpoint)
        .connect(http)
        .chat(&profile.model)
        .erase(),
        _ => return Err("模型协议插件未安装".into()),
    };
    Ok(ProviderModel::Http(model))
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
        let error =
            rig_core::ProviderError::from(rig_core::http_client::Error::Instance(Box::new(error)));
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
