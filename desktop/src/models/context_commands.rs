use super::{connections, context};
use crate::{assistant::config::Profile, database::Store};
use tauri::State;

pub async fn discover(profile: &Profile, key: &str) -> anyhow::Result<Option<context::Limits>> {
    use anyhow::{Context, ensure};
    let known = context::known(profile);
    if profile.adapter == "codex" {
        return Ok(None);
    }
    // Gemini models.get advertises capacities; a successful response may omit them.
    let entry = if profile.adapter == "gemini-native" {
        let id = profile
            .model
            .strip_prefix("models/")
            .unwrap_or(&profile.model);
        ensure!(
            !id.is_empty()
                && id
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-._".contains(&c)),
            "Gemini 模型 ID 无效"
        );
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(15))
            .build()?;
        let mut response = client
            .get(format!(
                "{}/v1beta/models/{id}",
                profile.endpoint.trim_end_matches('/')
            ))
            .header("x-goog-api-key", key)
            .send()
            .await
            .context("获取模型容量失败")?;
        if !response.status().is_success() {
            return Err(crate::app_error::http_error(response, "model_capacity")
                .await
                .into());
        }
        let mut bytes = Vec::new();
        while let Some(chunk) = response.chunk().await.context("读取模型容量失败")? {
            ensure!(bytes.len() + chunk.len() <= 1_048_576, "模型容量响应过大");
            bytes.extend_from_slice(&chunk);
        }
        Some(serde_json::from_slice(&bytes).context("模型容量响应不是有效 JSON")?)
    } else if known.is_none() {
        super::commands::fetch_models(profile, key)
            .await
            .map_err(anyhow::Error::msg)?
            .into_iter()
            .find(|row| super::commands::listed_model_matches(profile, row))
    } else {
        None
    };
    Ok(entry
        .as_ref()
        .and_then(|row| context::from_service(profile, row))
        .or(known))
}

#[tauri::command]
pub async fn resolve_text_model_limits(
    store: State<'_, Store>,
    id: String,
    model: String,
) -> Result<Option<context::Limits>, String> {
    let (service, secret) = {
        let db = store.db.lock().unwrap();
        (
            connections::get(&db, &id).map_err(|e| e.to_string())?,
            connections::key(&db, &id).map_err(|e| e.to_string())?,
        )
    };
    let mut profile = Profile {
        endpoint: service.endpoint,
        adapter: service.kind,
        model,
        ..Default::default()
    };
    crate::assistant::config::validate(&profile).map_err(|e| e.to_string())?;
    profile.limits.model_limits = discover(&profile, &secret)
        .await
        .map_err(|error| error.to_string())?;
    Ok(context::automatic(&profile))
}
