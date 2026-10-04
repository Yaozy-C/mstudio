use rig_agent::completion::PromptError;
use rig_core::completion::CompletionError;
use serde_json::Value;

pub fn describe(error: &PromptError, key: &str) -> String {
    if matches!(error, PromptError::PromptCancelled { .. }) {
        return crate::app_error::cancelled();
    }
    let status = error.provider_response_status().map(|s| s.as_u16());
    let code = match status {
        Some(401) => "AUTH_REQUIRED",
        Some(403) => "ACCESS_DENIED",
        Some(404) => "MODEL_UNAVAILABLE",
        Some(429) => "RATE_LIMITED",
        Some(500..=599) => "SERVICE_UNAVAILABLE",
        _ if matches!(
            error,
            PromptError::CompletionError(CompletionError::HttpError(_))
        ) && status.is_none() =>
        {
            "NETWORK_ERROR"
        }
        _ => "CHAT_FAILED",
    };
    let mut issue = crate::app_error::AppError::new(code, "chat", &describe_details(error, key));
    issue.http_status = status;
    issue.to_string()
}
fn describe_details(error: &PromptError, key: &str) -> String {
    match error {
        PromptError::CompletionError(CompletionError::ProviderError(message))
            if message.starts_with("Codex: ") => {
                return format!("{}。已完成的操作保留。", clean(message, key));
            }
        PromptError::PromptCancelled { reason, .. } => return reason.clone(),
        PromptError::MaxTurnsError { .. } => return "本轮已达到 16 个执行步骤；已完成的操作保留，可继续下一轮。".into(),
        PromptError::UnknownToolCall { .. } => return "模型请求了当前 Agent 未提供的工具；请重新描述任务或选择相应 Agent。已完成的操作保留。".into(),
        PromptError::CompletionError(CompletionError::HttpError(
            rig_core::http_client::Error::Instance(source),
        )) if source.downcast_ref::<rig_http::Error>().is_some_and(|e| e.is_timeout()) => {
            return "本次模型请求超时，请检查模型服务或稍后继续；已完成的操作保留。".into();
        }
        _ => {}
    }
    let status = error.provider_response_status().map(|s| s.as_u16());
    let reason = error
        .provider_response_json()
        .ok()
        .flatten()
        .and_then(provider_message);
    if reason
        .as_deref()
        .is_some_and(|v| v.contains("thought_signature") || v.contains("thoughtSignature"))
    {
        return "Gemini 工具调用签名缺失或无效（HTTP 400）；请在模型设置中使用 Gemini 原生协议。已完成的操作保留。".into();
    }
    let summary = match status {
        Some(401 | 403) => "模型服务拒绝认证，请检查 API Key 和权限",
        Some(404) => "模型或接口不存在，请核对模型 ID 和服务地址",
        Some(429) => "模型服务限流或额度不足，请检查额度后重试",
        Some(400 | 422) => "模型服务拒绝了请求",
        Some(500..=599) => "模型服务暂时异常，请稍后重试",
        Some(_) => "模型服务请求失败",
        None => match error {
            PromptError::CompletionError(CompletionError::HttpError(_)) => {
                "无法连接模型服务，请检查网络和服务地址"
            }
            PromptError::CompletionError(
                CompletionError::JsonError(_) | CompletionError::ResponseError(_),
            ) => "模型响应无法解析，请检查接口协议是否与服务一致",
            PromptError::CompletionError(CompletionError::RequestError(_)) => {
                "无法构造模型请求，请检查接口支持的资料类型与工具格式"
            }
            PromptError::MemoryError(_) => "读取对话上下文失败，请重试",
            _ => "模型执行失败，请重试",
        },
    };
    let code = status.map(|s| format!("（HTTP {s}）")).unwrap_or_default();
    let detail = reason
        .map(|s| clean(&s, key))
        .filter(|s| !s.is_empty())
        .map(|s| format!("：{s}"))
        .unwrap_or_default();
    format!("{summary}{code}{detail}。已完成的操作保留。")
}

pub fn context_overflow(error: &CompletionError) -> bool {
    if let CompletionError::ProviderError(message) = error {
        return message.starts_with("Codex: ")
            && (message.contains("文字上下文超过输入长度限制")
                || message.contains("Input exceeds the maximum length of 1048576 characters."));
    }
    let CompletionError::HttpError(http) = error else {
        return false;
    };
    use rig_core::http_client::Error;
    let (status, body) = match http {
        Error::InvalidStatusCode(status) => (status.as_u16(), None),
        Error::InvalidStatusCodeWithMessage(status, body) => (status.as_u16(), Some(body.as_str())),
        Error::InvalidStatusCodeWithDetails { status, body, .. } => {
            (status.as_u16(), Some(body.as_str()))
        }
        _ => return false,
    };
    if status == 413 {
        return true;
    }
    if !matches!(status, 400 | 422) {
        return false;
    }
    body.and_then(|body| serde_json::from_str(body).ok())
        .and_then(provider_message)
        .is_some_and(|message| {
            let message = message.to_ascii_lowercase();
            [
                "context length",
                "context window",
                "maximum context",
                "too many tokens",
                "input token limit",
                "prompt is too long",
                "token limit exceeded",
            ]
            .iter()
            .any(|phrase| message.contains(phrase))
        })
}

// Only show a provider's error message, never a raw request, response dump or signed reasoning.
fn provider_message(value: Value) -> Option<String> {
    let value = value.as_array().and_then(|v| v.first()).unwrap_or(&value);
    value
        .pointer("/error/message")
        .or_else(|| value.get("message"))
        .and_then(Value::as_str)
        .map(str::to_owned)
}
fn clean(message: &str, key: &str) -> String {
    let message = if key.is_empty() {
        message.into()
    } else {
        message.replace(key, "[密钥已隐藏]")
    };
    message
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .chars()
        .take(400)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig_core::http_client;
    use serde_json::json;
    fn failure(status: u16, body: Value) -> PromptError {
        CompletionError::HttpError(http_client::Error::InvalidStatusCodeWithMessage(
            status.to_string().parse().unwrap(),
            body.to_string(),
        ))
        .into()
    }
    #[test]
    fn extracts_only_message_and_redacts_credentials() {
        let body = json!([{"error":{"message":"invalid secret-key", "request":{"authorization":"secret-key"}}}]);
        assert_eq!(
            clean(&provider_message(body).unwrap(), "secret-key"),
            "invalid [密钥已隐藏]"
        );
        assert!(provider_message(json!({"request":{"secret":"hidden"}})).is_none());
        assert_eq!(clean(&"中".repeat(900), "").chars().count(), 400);
    }
    #[test]
    fn reports_http_reason_without_secrets_or_payloads() {
        let error = failure(
            400,
            json!([{"error":{"message":"missing thought_signature"}}]),
        );
        assert!(describe(&error, "").contains("Gemini 原生协议"));
        let error = failure(
            400,
            json!({"error":{"message":"unsupported parameter secret-key"},"request":{"prompt":"private prompt"}}),
        );
        let message = describe(&error, "secret-key");
        assert!(message.contains("HTTP 400"));
        assert!(message.contains("unsupported parameter [密钥已隐藏]"));
        assert!(!message.contains("private prompt"));
        for (status, expected) in [
            (401, "认证"),
            (404, "不存在"),
            (429, "额度"),
            (503, "暂时异常"),
        ] {
            assert!(describe(&failure(status, json!({})), "").contains(expected));
        }
    }
    #[test]
    fn authentication_classification_uses_status_not_provider_wording() {
        for text in ["arbitrary wording", "请选择模型", "no localized keywords"] {
            let value: Value = serde_json::from_str(&describe(
                &failure(401, json!({"error":{"message":text}})),
                "",
            ))
            .unwrap();
            assert_eq!(value["code"], "AUTH_REQUIRED");
            assert_eq!(value["httpStatus"], 401);
        }
    }
    #[test]
    fn parse_failure_is_distinct_from_network_failure() {
        let parse: PromptError = CompletionError::ResponseError("invalid payload".into()).into();
        assert!(describe(&parse, "").contains("无法解析"));
        assert!(!describe(&failure(400, json!({})), "").contains("secret"));
    }
    #[test]
    fn codex_failure_keeps_its_reason_instead_of_claiming_a_protocol_mismatch() {
        let error: PromptError =
            CompletionError::ProviderError("Codex: 连接 Codex 超时".into()).into();
        let message = describe(&error, "");
        assert!(message.contains("连接 Codex 超时"));
        assert!(!message.contains("接口协议"));
        assert!(context_overflow(&CompletionError::ProviderError(
            "Codex: Codex 文字上下文超过输入长度限制，请压缩对话后继续".into()
        )));
        assert!(!context_overflow(&CompletionError::ProviderError(
            "Codex: 连接 Codex 超时".into()
        )));
    }
}
