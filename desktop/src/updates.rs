use serde_json::{Value, json};
use std::time::Duration;

const RELEASE_API: &str = "https://api.github.com/repos/Yaozy-C/mstudio/releases/latest";
const RELEASE_PAGE: &str = "https://github.com/Yaozy-C/mstudio/releases";

#[tauri::command]
pub async fn check_app_update() -> Result<Value, String> {
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(15))
        .user_agent(concat!("Mstudio/", env!("CARGO_PKG_VERSION")))
        .build()
        .map_err(|_| "UPDATE_CHECK_FAILED".to_owned())?;
    let response = client
        .get(RELEASE_API)
        .header("Accept", "application/vnd.github+json")
        .send()
        .await
        .map_err(|_| "UPDATE_CHECK_FAILED".to_owned())?;
    let release = if response.status() == reqwest::StatusCode::NOT_FOUND {
        Value::Null
    } else {
        if !response.status().is_success() {
            return Err("UPDATE_CHECK_FAILED".into());
        }
        let body: Value = response.json().await.map_err(|_| "UPDATE_CHECK_FAILED")?;
        json!({
            "tag_name": body["tag_name"],
            "draft": body["draft"],
            "prerelease": body["prerelease"],
        })
    };
    Ok(json!({ "currentVersion": env!("CARGO_PKG_VERSION"), "release": release }))
}

// Only the project's fixed release page can be opened by this command.
#[tauri::command]
pub fn open_release_page() -> Result<(), String> {
    #[cfg(target_os = "macos")]
    let result = std::process::Command::new("open").arg(RELEASE_PAGE).spawn();
    #[cfg(target_os = "windows")]
    let result = std::process::Command::new("rundll32")
        .args(["url.dll,FileProtocolHandler", RELEASE_PAGE])
        .spawn();
    #[cfg(not(any(target_os = "macos", target_os = "windows")))]
    let result = std::process::Command::new("xdg-open")
        .arg(RELEASE_PAGE)
        .spawn();
    result
        .map(|_| ())
        .map_err(|_| "RELEASE_PAGE_FAILED".to_owned())
}
