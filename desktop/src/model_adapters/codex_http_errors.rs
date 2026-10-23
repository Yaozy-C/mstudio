use crate::app_error::AppError;
use serde_json::Value;

pub(super) fn transport(error: reqwest::Error) -> AppError {
    let reason = if error.is_timeout() {
        "timeout"
    } else if error.is_connect() {
        "connect"
    } else if error.is_body() {
        "body"
    } else if error.is_decode() {
        "decode"
    } else if error.is_request() {
        "request"
    } else {
        "transport"
    };
    let mut issue = AppError::new(
        "NETWORK_ERROR",
        "submit",
        &format!("Codex 图片传输失败（{reason}）；未自动重试，服务端可能仍在生成"),
    );
    issue.outcome = Some("unknown".into());
    issue
}

fn details(status: u16, value: &Value) -> String {
    let guidance = match status {
        401 => "登录已失效，请重新登录",
        403 => "接口拒绝访问，请检查账号权限",
        429 => "额度不足或请求过于频繁",
        400 | 404 | 422 => "请检查模型 ID、账号权限、描述和参考图片；未切换型号",
        _ => "请检查生成服务状态",
    };
    let mut details = format!("Codex 图片请求失败（HTTP {status}）：{guidance}；未自动重试");
    // These fields are identifiers, not arbitrary provider messages. A strict
    // vocabulary prevents error bodies from echoing prompts or credentials.
    for (field, allowed) in [
        (
            "code",
            &[
                "model_not_found",
                "invalid_model",
                "unsupported_model",
                "invalid_api_key",
                "invalid_request",
                "invalid_value",
                "invalid_image",
                "image_parse_error",
                "content_policy_violation",
                "moderation_blocked",
                "insufficient_quota",
                "rate_limit_exceeded",
                "server_error",
                "access_denied",
                "account_deactivated",
            ][..],
        ),
        (
            "type",
            &[
                "invalid_request_error",
                "authentication_error",
                "permission_error",
                "rate_limit_error",
                "server_error",
                "insufficient_quota",
            ][..],
        ),
        (
            "param",
            &[
                "model",
                "prompt",
                "image",
                "images",
                "n",
                "output_format",
                "size",
                "quality",
                "background",
                "moderation",
                "partial_images",
            ][..],
        ),
    ] {
        if let Some(identifier) = value["error"][field]
            .as_str()
            .filter(|v| allowed.contains(v))
        {
            details.push_str(&format!("；{field}={identifier}"));
        }
    }
    details
}

pub(super) async fn response(mut response: reqwest::Response) -> AppError {
    let status = response.status().as_u16();
    let mut bytes = Vec::new();
    while let Ok(Some(chunk)) = response.chunk().await {
        if bytes.len() + chunk.len() > 32_768 {
            break;
        }
        bytes.extend_from_slice(&chunk);
    }
    let value = serde_json::from_slice::<Value>(&bytes).unwrap_or(Value::Null);
    let mut issue = AppError::http(status, "submit", &details(status, &value));
    if matches!(
        value["error"]["code"].as_str(),
        Some("content_policy_violation" | "moderation_blocked")
    ) {
        issue.code = "CONTENT_REJECTED".into();
    }
    issue
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn http_diagnostics_keep_identifiers_and_exclude_echoed_request_data() {
        let detail = details(
            404,
            &json!({"error":{
                "message":"private prompt Bearer fake-token", "code":"model_not_found",
                "type":"invalid_request_error", "param":"model", "input":{"token":"secret"}
            }}),
        );
        for expected in [
            "HTTP 404",
            "code=model_not_found",
            "type=invalid_request_error",
            "param=model",
        ] {
            assert!(detail.contains(expected));
        }
        for private in ["private prompt", "fake-token", "secret"] {
            assert!(!detail.contains(private));
        }
        let detail = details(
            422,
            &json!({"error":{"code":"secret", "type":"private", "param":"token"}}),
        );
        for private in ["secret", "private", "token"] {
            assert!(!detail.contains(private));
        }
    }
}
