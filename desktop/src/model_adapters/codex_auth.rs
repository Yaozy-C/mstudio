//! Read only the file-backed Codex login; refresh remains owned by Codex.
use super::codex_rpc::Rpc;
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::URL_SAFE_NO_PAD};
use serde_json::{Value, json};
use std::{
    path::PathBuf,
    time::{Duration, SystemTime, UNIX_EPOCH},
};

// Deliberately no Debug/Serialize: credentials must never enter job state or logs.
pub(super) struct Credentials {
    pub token: String,
    pub account: String,
}
fn auth_path() -> Result<PathBuf> {
    let home = std::env::var_os("CODEX_HOME")
        .map(PathBuf::from)
        .or_else(|| {
            std::env::var_os("HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .map(|home| PathBuf::from(home).join(".codex"))
        })
        .context("无法定位 Codex 登录目录")?;
    Ok(home.join("auth.json"))
}
fn read() -> Result<Credentials> {
    let bytes = std::fs::read(auth_path()?).map_err(|_| anyhow::anyhow!(
        "无法读取 Codex 文件登录；请使用本机 Codex 文件凭据登录。直连暂不支持仅存于系统钥匙串的登录"
    ))?;
    parse(&bytes)
}
fn parse(bytes: &[u8]) -> Result<Credentials> {
    let value: Value =
        serde_json::from_slice(bytes).map_err(|_| anyhow::anyhow!("Codex 登录文件无效"))?;
    ensure!(
        value["auth_mode"].is_null() || value["auth_mode"] == "chatgpt",
        "请使用 ChatGPT 账号登录 Codex"
    );
    let token = value["tokens"]["access_token"]
        .as_str()
        .filter(|s| !s.is_empty())
        .context("缺少 Codex 登录令牌，请重新登录")?;
    let account = value["tokens"]["account_id"]
        .as_str()
        .filter(|s| !s.is_empty())
        .context("缺少 Codex 账号信息，请重新登录")?;
    Ok(Credentials {
        token: token.into(),
        account: account.into(),
    })
}
fn needs_refresh(token: &str, now: u64) -> bool {
    token
        .split('.')
        .nth(1)
        .and_then(|p| URL_SAFE_NO_PAD.decode(p).ok())
        .and_then(|b| serde_json::from_slice::<Value>(&b).ok())
        .and_then(|v| v["exp"].as_u64())
        .is_none_or(|exp| exp <= now.saturating_add(60))
}
pub(crate) fn check_login() -> Result<()> {
    read().map(|_| ())
}
pub(super) async fn credentials() -> Result<Credentials> {
    let current = read()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if !needs_refresh(&current.token, now) {
        return Ok(current);
    }
    // Serialize refreshes so concurrent image jobs do not rotate the same token.
    static REFRESH: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());
    let _guard = REFRESH.lock().await;
    let current = read()?;
    let now = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs();
    if !needs_refresh(&current.token, now) {
        return Ok(current);
    }
    tokio::time::timeout(Duration::from_secs(30), async {
        let mut rpc = Rpc::start_text().await?;
        let result = rpc.call("account/read", json!({"refreshToken":true})).await;
        rpc.stop().await;
        let account = result?;
        ensure!(account["account"]["type"] == "chatgpt", "请重新登录 Codex");
        anyhow::Ok(())
    })
    .await
    .map_err(|_| anyhow::anyhow!("Codex 登录刷新超时，请重试"))?
    .map_err(|_| anyhow::anyhow!("Codex 登录刷新失败，请重新登录"))?;
    let refreshed = read()?;
    ensure!(
        refreshed.account == current.account,
        "Codex 账号已切换，请重新提交任务"
    );
    ensure!(
        !needs_refresh(&refreshed.token, now),
        "Codex 登录已过期，请重新登录"
    );
    Ok(refreshed)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn accepts_chatgpt_credentials_and_checks_expiration_without_disclosing_secrets() {
        let token = format!(
            "header.{}.signature",
            URL_SAFE_NO_PAD.encode(br#"{"exp":1000}"#)
        );
        let value =
            json!({"auth_mode":"chatgpt","tokens":{"access_token":token,"account_id":"test"}});
        let c = parse(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert!(!needs_refresh(&c.token, 900));
        assert!(needs_refresh(&c.token, 940));
        assert!(needs_refresh("invalid", 0));
        assert!(parse(br#"{"auth_mode":"apikey","OPENAI_API_KEY":"secret"}"#).is_err());
        assert!(
            !parse(b"secret")
                .err()
                .unwrap()
                .to_string()
                .contains("secret")
        );
    }
}
