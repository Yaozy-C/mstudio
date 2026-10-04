use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Stable wire format, encoded as JSON in existing String error fields for old projects.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: String,
    pub retryable: bool,
    pub stage: String,
    pub outcome: Option<String>,
    pub http_status: Option<u16>,
    pub details: String,
}
impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&serde_json::to_string(self).map_err(|_| std::fmt::Error)?)
    }
}
impl std::error::Error for AppError {}
impl AppError {
    pub fn new(code: &str, stage: &str, details: &str) -> Self {
        Self {
            code: code.into(),
            retryable: false,
            stage: stage.into(),
            outcome: None,
            http_status: None,
            details: details.chars().take(2000).collect(),
        }
    }
    pub fn http(status: u16, stage: &str, details: &str) -> Self {
        let code = match status {
            400 | 404 | 409 if stage == "cancel" => "JOB_CANCEL_FAILED",
            404 if stage == "submit" => "MODEL_UNAVAILABLE",
            400 | 422 => "INVALID_INPUT",
            401 => "AUTH_REQUIRED",
            402 => "QUOTA_EXCEEDED",
            403 => "ACCESS_DENIED",
            404 => "JOB_NOT_FOUND",
            429 => "RATE_LIMITED",
            500..=599 => "SERVICE_UNAVAILABLE",
            _ => "OPERATION_FAILED",
        };
        let mut error = Self::new(code, stage, details);
        error.http_status = Some(status);
        error.retryable = status == 429 || status >= 500;
        // Only an explicit client rejection proves submission did not start.
        if stage == "submit" && (400..500).contains(&status) && !matches!(status, 408 | 409) {
            error.outcome = Some("rejected".into());
        }
        error
    }
    pub fn from_error(error: &anyhow::Error, code: &str, stage: &str) -> Self {
        if let Some(known) = error.downcast_ref::<Self>() {
            return known.clone();
        }
        let mut result = Self::new(code, stage, &error.to_string());
        if let Some(network) = error.downcast_ref::<reqwest::Error>() {
            result.code = "NETWORK_ERROR".into();
            result.retryable = true;
            result.details = if network.is_timeout() {
                "服务请求超时"
            } else {
                "服务连接中断"
            }
            .into();
        }
        result
    }
}
pub fn rejected(error: impl std::fmt::Display) -> String {
    let mut error = AppError::new("VALIDATION_FAILED", "prepare", &error.to_string());
    error.outcome = Some("rejected".into());
    error.to_string()
}
pub fn wire(error: anyhow::Error, code: &str, stage: &str) -> String {
    AppError::from_error(&error, code, stage).to_string()
}
/// Keep only diagnostic messages/field names, not provider request inputs or headers.
pub fn provider_details(value: &Value) -> String {
    let detail = &value["detail"];
    if let Some(items) = detail.as_array() {
        return items
            .iter()
            .take(10)
            .map(|item| {
                let field = item["loc"]
                    .as_array()
                    .map(|v| {
                        v.iter()
                            .filter_map(Value::as_str)
                            .collect::<Vec<_>>()
                            .join(".")
                    })
                    .unwrap_or_default();
                format!("{}: {}", field, item["msg"].as_str().unwrap_or("参数无效"))
            })
            .collect::<Vec<_>>()
            .join("\n")
            .chars()
            .take(2000)
            .collect();
    }
    [
        detail.as_str(),
        value["message"].as_str(),
        value["error"]["message"].as_str(),
        value["error"].as_str(),
    ]
    .into_iter()
    .flatten()
    .next()
    .unwrap_or("服务未提供可读的错误原因")
    .chars()
    .take(2000)
    .collect()
}
pub async fn http_error(mut response: reqwest::Response, stage: &str) -> AppError {
    let status = response.status().as_u16();
    let mut bytes = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        if bytes.len() + chunk.len() > 32_768 {
            break;
        }
        bytes.extend_from_slice(&chunk);
    }
    let value = serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null);
    let mut error = AppError::http(status, stage, &provider_details(&value));
    if value["error_type"] == "client_cancelled" {
        error.code = "JOB_CANCELLED".into();
        error.outcome = Some("cancelled".into());
    }
    if value["detail"].as_array().is_some_and(|items| {
        items.iter().any(|v| {
            matches!(
                v["type"].as_str(),
                Some("content_policy_violation" | "content_policy_error")
            )
        })
    }) {
        error.code = "CONTENT_REJECTED".into();
    }
    error
}

pub fn cancelled() -> String {
    let mut error = AppError::new(
        "CHAT_STOPPED",
        "chat",
        "Response stopped; completed operations retained",
    );
    error.outcome = Some("cancelled".into());
    error.to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejection_is_not_inferred_from_polling_or_server_failure() {
        assert_eq!(
            AppError::http(422, "submit", "bad duration")
                .outcome
                .as_deref(),
            Some("rejected")
        );
        assert_eq!(AppError::http(422, "status", "bad").outcome, None);
        assert_eq!(AppError::http(503, "submit", "busy").outcome, None);
        assert!(!AppError::http(401, "status", "auth").retryable);
    }
    #[test]
    fn validation_details_do_not_include_input() {
        let detail = provider_details(
            &serde_json::json!({"detail":[{"loc":["body","duration"],"msg":"unsupported duration","input":"private prompt"}]}),
        );
        assert_eq!(detail, "body.duration: unsupported duration");
        let e: anyhow::Error = AppError::http(422, "submit", &detail).into();
        let encoded = wire(e.context("request failed"), "OPERATION_FAILED", "submit");
        assert!(encoded.contains("INVALID_INPUT"));
        assert!(!encoded.contains("private prompt"));
    }
}
