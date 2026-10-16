use super::{connections, context};
use crate::{assistant::config::Profile, database::Store};
use tauri::State;

pub async fn discover(profile: &Profile, key: &str) -> Option<context::Limits> {
    let known = context::known(profile);
    if profile.adapter == "codex" {
        return None;
    }
    // Standard OpenAI /models has no capacities; its verified catalog is authoritative.
    // Gemini models.get carries inputTokenLimit/outputTokenLimit, including new models.
    let entry = if profile.adapter == "gemini-native" {
        let id = profile
            .model
            .strip_prefix("models/")
            .unwrap_or(&profile.model);
        if !id
            .bytes()
            .all(|c| c.is_ascii_alphanumeric() || b"-._".contains(&c))
            || id.is_empty()
        {
            return known;
        }
        let client = reqwest::Client::builder()
            .redirect(reqwest::redirect::Policy::none())
            .timeout(std::time::Duration::from_secs(15))
            .build()
            .ok()?;
        let response = client
            .get(format!(
                "{}/v1beta/models/{id}",
                profile.endpoint.trim_end_matches('/')
            ))
            .header("x-goog-api-key", key)
            .send()
            .await;
        match response {
            Ok(response) if response.status().is_success() => {
                // Metadata responses are small; do not accept an unbounded provider body.
                let mut response = response;
                let mut bytes = Vec::new();
                while let Ok(Some(chunk)) = response.chunk().await {
                    if bytes.len() + chunk.len() > 1_048_576 {
                        return known;
                    }
                    bytes.extend_from_slice(&chunk);
                }
                serde_json::from_slice(&bytes).ok()
            }
            _ => None,
        }
    } else if known.is_none() {
        super::commands::fetch_models(profile, key)
            .await
            .ok()
            .and_then(|rows| {
                rows.into_iter()
                    .find(|row| super::commands::listed_model_matches(profile, row))
            })
    } else {
        None
    };
    entry
        .as_ref()
        .and_then(|row| context::from_service(profile, row))
        .or(known)
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
    profile.limits.model_limits = discover(&profile, &secret).await;
    Ok(context::automatic(&profile))
}
