//! Optional authentication headers declared by service configuration.
use anyhow::{Context, Result};
use reqwest::header::{HeaderMap, HeaderName, HeaderValue};

pub fn headers(endpoint: &str, key: &str) -> Result<HeaderMap> {
    let config: serde_json::Value = serde_json::from_str(include_str!(
        "../../../frontend/src/models/config/text-transports.json"
    ))?;
    let mut headers = HeaderMap::new();
    if let Some(name) = config[endpoint.trim_end_matches('/')]["apiKeyHeader"].as_str() {
        let name = HeaderName::from_bytes(name.as_bytes()).context("服务认证字段无效")?;
        let mut value = HeaderValue::from_str(key).context("服务认证值无效")?;
        value.set_sensitive(true);
        headers.insert(name, value);
    }
    Ok(headers)
}
