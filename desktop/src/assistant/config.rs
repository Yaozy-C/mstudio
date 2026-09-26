use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Profile {
    pub endpoint: String,
    pub model: String,
    pub adapter: String,
    #[serde(default)]
    pub context_window: Option<usize>,
    pub inputs: Inputs,
}
pub fn validate(profile: &Profile) -> Result<()> {
    ensure!(
        !profile.endpoint.trim().is_empty(),
        "请先在模型中心连接 Agent 服务"
    );
    let url = reqwest::Url::parse(&profile.endpoint)?;
    let local = matches!(url.host_str(), Some("localhost" | "127.0.0.1" | "[::1]"));
    ensure!(
        url.scheme() == "https" || (local && url.scheme() == "http"),
        "远程模型必须使用 HTTPS；本机模型可用 HTTP"
    );
    ensure!(
        url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "模型地址不能包含密钥、查询参数或用户名"
    );
    if url.host_str() == Some("generativelanguage.googleapis.com") {
        ensure!(
            profile.adapter == "gemini-native",
            "Google Gemini 请使用 Gemini 原生协议，以完整保留工具调用签名"
        );
        ensure!(
            url.path().trim_matches('/').is_empty(),
            "Gemini 原生地址请填写 https://generativelanguage.googleapis.com，不附加 /v1beta 或 /openai"
        );
    }
    ensure!(
        !profile.model.trim().is_empty() && profile.model.len() < 256,
        "请填写模型 ID"
    );
    ensure!(
        profile
            .context_window
            .is_none_or(|n| (8192..=2_000_000).contains(&n)),
        "上下文窗口须在 8192 到 2000000 token 之间"
    );
    ensure!(
        [
            "openai-compatible",
            "openai-responses",
            "gemini-native",
            "anthropic-native"
        ]
        .contains(&profile.adapter.as_str()),
        "不支持的模型协议"
    );
    Ok(())
}

impl Profile {
    pub fn context_window(&self) -> usize {
        self.context_window.unwrap_or(32_768)
    }
    pub fn input_budget(&self) -> usize {
        self.context_window().saturating_mul(4) / 5
    }
    pub fn retain_budget(&self) -> usize {
        self.context_window().saturating_mul(16) / 100
    }
}

pub fn is_local(endpoint: &str) -> bool {
    reqwest::Url::parse(endpoint)
        .ok()
        .is_some_and(|u| matches!(u.host_str(), Some("localhost" | "127.0.0.1" | "[::1]")))
}

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Inputs {
    #[serde(default)]
    pub image: bool,
    #[serde(default)]
    pub audio: bool,
    #[serde(default)]
    pub video: bool,
    #[serde(default)]
    pub document: bool,
}

#[cfg(test)]
mod tests {
    use super::{Profile, validate};
    use serde_json::json;

    #[test]
    fn model_requires_explicit_input_capabilities() {
        let mut value = json!({
            "endpoint":"https://example.com", "model":"test",
            "adapter":"openai-compatible", "vision":true
        });
        assert!(serde_json::from_value::<Profile>(value.clone()).is_err());
        value["inputs"] = json!({"image":false,"audio":true,"video":false,"document":false});
        let profile: Profile = serde_json::from_value(value).unwrap();
        assert!(!profile.inputs.image);
        assert!(profile.inputs.audio);
    }
    #[test]
    fn official_gemini_rejects_incompatible_protocol_before_sending() {
        let mut profile = Profile {
            endpoint: "https://generativelanguage.googleapis.com/v1beta/openai".into(),
            model: "gemini-test".into(),
            adapter: "openai-compatible".into(),
            context_window: None,
            inputs: Default::default(),
        };
        assert!(
            validate(&profile)
                .unwrap_err()
                .to_string()
                .contains("原生协议")
        );
        profile.adapter = "gemini-native".into();
        assert!(
            validate(&profile)
                .unwrap_err()
                .to_string()
                .contains("不附加")
        );
        profile.endpoint = "https://generativelanguage.googleapis.com".into();
        validate(&profile).unwrap();
        profile.endpoint = "https://proxy.example.com/v1".into();
        profile.adapter = "openai-compatible".into();
        validate(&profile).unwrap();
    }
}
