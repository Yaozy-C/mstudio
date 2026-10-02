//! User-facing setup states; credentials stay with Codex.
use super::{codex_discovery, codex_rpc::Rpc};
use serde_json::{Value, json};
use std::time::Duration;

fn state(status: &str) -> Value {
    json!({"status": status, "loggedIn": status == "ready" || status == "unsupported", "imageGeneration": status == "ready"})
}

#[tauri::command]
pub async fn codex_image_status() -> Value {
    if codex_discovery::binaries().await.is_empty() {
        return state("missing");
    }
    let result = tokio::time::timeout(Duration::from_secs(30), async {
        let mut rpc = Rpc::start().await?;
        let result: anyhow::Result<Value> = async {
            let account = rpc
                .call("account/read", json!({"refreshToken":false}))
                .await?;
            if account["account"]["type"] != "chatgpt" {
                return Ok(state("login_required"));
            }
            let capabilities = rpc
                .call("modelProvider/capabilities/read", json!({}))
                .await?;
            Ok(state(if capabilities["imageGeneration"] == true {
                "ready"
            } else {
                "unsupported"
            }))
        }
        .await;
        rpc.stop().await;
        result
    })
    .await;
    match result {
        Ok(Ok(value)) => value,
        Ok(Err(_)) => state("unavailable"),
        Err(_) => state("timeout"),
    }
}

fn open_browser(url: &str) -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(url).spawn();
    #[cfg(windows)]
    let result = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", url])
        .spawn();
    #[cfg(not(any(target_os = "macos", windows)))]
    let result = std::process::Command::new("xdg-open").arg(url).spawn();
    result.map(|_| ()).map_err(|_| "无法打开浏览器".into())
}

#[tauri::command]
pub fn open_codex_download() -> Result<(), String> {
    open_browser("https://learn.chatgpt.com/docs/codex/cli")
}

fn login_url_allowed(url: &str) -> bool {
    reqwest::Url::parse(url).is_ok_and(|url| {
        url.scheme() == "https"
            && url.username().is_empty()
            && url.password().is_none()
            && matches!(
                url.host_str(),
                Some("auth.openai.com" | "auth0.openai.com" | "chatgpt.com")
            )
    })
}

#[tauri::command]
pub async fn codex_login() -> Result<(), String> {
    tokio::time::timeout(Duration::from_secs(180), async {
        let mut rpc = Rpc::start_text().await?;
        let result: anyhow::Result<()> = async {
            let login = rpc
                .call("account/login/start", json!({"type":"chatgpt"}))
                .await?;
            let url = login["authUrl"]
                .as_str()
                .ok_or_else(|| anyhow::anyhow!("Codex 未返回登录地址"))?;
            anyhow::ensure!(login_url_allowed(url), "Codex 返回了无法识别的登录地址");
            open_browser(url).map_err(anyhow::Error::msg)?;
            loop {
                let event = rpc.next().await?;
                if event["method"] == "account/login/completed" {
                    anyhow::ensure!(event["params"]["success"] == true, "登录未完成，请重试");
                    return Ok(());
                }
                rpc.decline(&event).await?;
            }
        }
        .await;
        rpc.stop().await;
        result
    })
    .await
    .map_err(|_| "登录超时，请重试".to_owned())?
    .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[tokio::test]
    #[ignore = "requires local Codex and ChatGPT login; no image is generated"]
    async fn installed_codex_is_ready_without_generating() {
        assert_eq!(codex_image_status().await["status"], "ready");
    }
    #[test]
    fn login_only_opens_official_https_origins() {
        assert!(login_url_allowed(
            "https://auth.openai.com/authorize?state=test"
        ));
        for url in [
            "http://auth.openai.com",
            "https://auth.openai.com.evil.test",
            "file:///tmp/test",
            "https://user@chatgpt.com",
        ] {
            assert!(!login_url_allowed(url));
        }
    }
}
