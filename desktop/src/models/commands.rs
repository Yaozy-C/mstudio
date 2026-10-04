use super::{Catalog, Model, storage};
use crate::database::Store;
use anyhow::Result;
use tauri::State;

#[tauri::command]
pub fn model_catalog(store: State<Store>, project_id: Option<String>) -> Result<Catalog, String> {
    storage::catalog(&store.db.lock().unwrap(), project_id.as_deref()).map_err(|e| e.to_string())
}

fn transaction(
    store: &Store,
    action: impl FnOnce(&rusqlite::Connection) -> Result<()>,
) -> Result<(), String> {
    (|| -> Result<()> {
        let mut db = store.db.lock().unwrap();
        let tx = db.transaction()?;
        action(&tx)?;
        tx.commit()?;
        Ok(())
    })()
    .map_err(|e| crate::app_error::wire(e, "VALIDATION_FAILED", "model_settings"))
}

#[tauri::command]
pub fn save_model(
    store: State<Store>,
    profile: Model,
    key: Option<String>,
    clear_key: bool,
) -> Result<(), String> {
    transaction(&store, |db| storage::save(db, profile, key, clear_key))
}

#[tauri::command]
pub fn remove_model(store: State<Store>, id: String) -> Result<(), String> {
    transaction(&store, |db| storage::remove(db, &id))
}

#[tauri::command]
pub fn default_agent_model(store: State<Store>, id: String) -> Result<(), String> {
    transaction(&store, |db| storage::set_default(db, &id))
}

#[tauri::command]
pub fn select_conversation_model(
    store: State<Store>,
    project_id: String,
    id: Option<String>,
) -> Result<(), String> {
    transaction(&store, |db| storage::select(db, &project_id, id.as_deref()))
}

#[tauri::command]
pub async fn test_model(store: State<'_, Store>, id: String) -> Result<String, String> {
    let (model, key) =
        storage::resolve(&store.db.lock().unwrap(), None, Some(&id)).map_err(|e| e.to_string())?;
    check_connection(&model, &key).await
}

pub(super) async fn check_connection(model: &Model, key: &str) -> Result<String, String> {
    let list = fetch_models(&model.profile, key).await?;
    Ok(
        if list
            .iter()
            .any(|entry| listed_model_matches(&model.profile, entry))
        {
            "连接正常，已找到所选模型。对话和工具能力以实际调用为准。".into()
        } else {
            "连接正常，但列表中未找到此模型 ID，请核对名称。".into()
        },
    )
}
pub(super) async fn fetch_models(
    profile: &crate::assistant::config::Profile,
    key: &str,
) -> Result<Vec<serde_json::Value>, String> {
    if profile.adapter == "codex" {
        return super::codex_connection::model_list()
            .await
            .map_err(|e| e.to_string());
    }
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .redirect(reqwest::redirect::Policy::none())
        .build()
        .map_err(|_| "无法创建模型连接")?;
    let endpoint = profile.endpoint.trim_end_matches('/');
    let request = match profile.adapter.as_str() {
        "gemini-native" => client
            .get(format!("{endpoint}/v1beta/models"))
            .header("x-goog-api-key", key),
        "anthropic-native" => client
            .get(format!("{endpoint}/models"))
            .header("x-api-key", key)
            .header("anthropic-version", "2023-06-01"),
        _ => client.get(format!("{endpoint}/models")).bearer_auth(key),
    };
    let mut response = request.send().await.map_err(|_| {
        crate::app_error::AppError::new("NETWORK_ERROR", "model_discovery", "Connection failed")
            .to_string()
    })?;
    if !response.status().is_success() {
        return Err(match response.status().as_u16() {
            401 | 403 => crate::app_error::AppError::http(
                response.status().as_u16(),
                "model_discovery",
                "Authentication rejected",
            )
            .to_string(),
            404 | 405 => "服务未提供 /models 接口，可保存后在 Agent 中验证对话".into(),
            code => format!("服务返回 HTTP {code}，请检查连接配置"),
        });
    }
    let mut bytes = Vec::new();
    while let Some(chunk) = response.chunk().await.map_err(|_| "读取模型列表失败")? {
        if bytes.len() + chunk.len() > 1_048_576 {
            return Err("模型列表过大".into());
        }
        bytes.extend_from_slice(&chunk);
    }
    let body: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_| "服务未返回有效的模型列表")?;
    let list = body["data"]
        .as_array()
        .or_else(|| body["models"].as_array())
        .ok_or("服务未返回兼容的模型列表")?;
    Ok(list.clone())
}

// Gemini's native and OpenAI endpoints return resource names (models/<id>).
// Other providers may use namespaced IDs literally, so preserve their full IDs.
pub(super) fn listed_model_matches(
    profile: &crate::assistant::config::Profile,
    entry: &serde_json::Value,
) -> bool {
    let Some(listed) = entry["id"].as_str().or_else(|| entry["name"].as_str()) else {
        return false;
    };
    let gemini = profile.adapter == "gemini-native"
        || reqwest::Url::parse(&profile.endpoint)
            .is_ok_and(|url| url.host_str() == Some("generativelanguage.googleapis.com"));
    if gemini {
        listed.strip_prefix("models/").unwrap_or(listed)
            == profile
                .model
                .strip_prefix("models/")
                .unwrap_or(&profile.model)
    } else {
        listed == profile.model
    }
}
