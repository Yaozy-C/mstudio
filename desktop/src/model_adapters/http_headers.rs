use anyhow::{Context, Result, ensure};
use reqwest::header::{HeaderName, HeaderValue};
use serde_json::Value;

const MAX_HEADERS: usize = 8;
const MAX_HEADER_NAME: usize = 64;
const MAX_HEADER_VALUE: usize = 1024;
// Credentials and transport framing stay under the adapter and service connection.
const RESERVED_HEADERS: [&str; 10] = [
    "authorization",
    "proxy-authorization",
    "host",
    "content-length",
    "connection",
    "cookie",
    "set-cookie",
    "transfer-encoding",
    "expect",
    "upgrade",
];
/// Vendor protocols may need documented request headers, such as DashScope's async
/// opt-in. Keys stay declarative; the API key is still sent from the service connection.
pub fn header_pairs(config: &Value) -> Result<Vec<(HeaderName, HeaderValue)>> {
    let Some(value) = config.get("headers").filter(|value| !value.is_null()) else {
        return Ok(Vec::new());
    };
    let object = value.as_object().context("headers 必须是 JSON 对象")?;
    ensure!(
        object.len() <= MAX_HEADERS,
        "自定义请求头最多 {MAX_HEADERS} 个"
    );
    let mut headers = Vec::new();
    for (name, value) in object {
        ensure!(
            !name.is_empty()
                && name.len() <= MAX_HEADER_NAME
                && name
                    .bytes()
                    .all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'_'),
            "请求头名称无效：{name}"
        );
        ensure!(
            !RESERVED_HEADERS.contains(&name.to_ascii_lowercase().as_str()),
            "请求头 {name} 由服务连接管理，不能在此配置"
        );
        let text = value
            .as_str()
            .with_context(|| format!("请求头 {name} 的值必须是字符串"))?;
        ensure!(
            !text.is_empty()
                && text.len() <= MAX_HEADER_VALUE
                && text.bytes().all(|b| (0x20..0x7f).contains(&b)),
            "请求头 {name} 的值无效"
        );
        headers.push((
            HeaderName::from_bytes(name.as_bytes())
                .with_context(|| format!("请求头名称无效：{name}"))?,
            HeaderValue::from_str(text).with_context(|| format!("请求头 {name} 的值无效"))?,
        ));
    }
    Ok(headers)
}
pub(super) fn with_headers(
    builder: reqwest::RequestBuilder,
    config: &Value,
) -> Result<reqwest::RequestBuilder> {
    Ok(header_pairs(config)?
        .into_iter()
        .fold(builder, |builder, (name, value)| {
            builder.header(name, value)
        }))
}
