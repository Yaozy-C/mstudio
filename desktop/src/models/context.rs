//! Adapter-owned capacities, distinct from user overrides and request output caps.
use crate::assistant::config::Profile;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::sync::OnceLock;

pub const UNKNOWN_CONTEXT: usize = 262_144;
pub const DEFAULT_HEADROOM: usize = 65_536;

#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Settings {
    #[serde(default)]
    pub context_window_mode: Option<Mode>,
    #[serde(default)]
    pub model_limits: Option<Limits>,
    #[serde(default)]
    pub max_output_tokens: Option<usize>,
    #[serde(default)]
    pub compaction_headroom: Option<usize>,
}

#[derive(Clone, Copy, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    Auto,
    Manual,
}

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Limits {
    pub endpoint: String,
    pub model: String,
    pub context_window: usize,
    pub max_output_tokens: Option<usize>,
    pub default_max_tokens: Option<usize>,
    pub source: String,
    pub source_url: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Entry {
    host: String,
    models: Vec<String>,
    context_window: usize,
    max_output_tokens: usize,
    default_max_tokens: usize,
    source: String,
}

fn model_id(model: &str) -> &str {
    model.trim().strip_prefix("models/").unwrap_or(model.trim())
}

pub fn known(profile: &Profile) -> Option<Limits> {
    if profile.adapter == "codex" {
        return None;
    }
    let url = reqwest::Url::parse(&profile.endpoint).ok()?;
    if url.scheme() != "https" || url.port().is_some() {
        return None;
    }
    static CATALOG: OnceLock<Vec<Entry>> = OnceLock::new();
    let entries = CATALOG.get_or_init(|| {
        serde_json::from_str(include_str!(
            "../../../frontend/src/models/config/text-context-limits.json"
        ))
        .expect("valid bundled model capacities")
    });
    let entry = entries.iter().find(|entry| {
        Some(entry.host.as_str()) == url.host_str()
            && entry.models.iter().any(|id| id == model_id(&profile.model))
    })?;
    Some(Limits {
        endpoint: profile.endpoint.trim_end_matches('/').into(),
        model: profile.model.clone(),
        context_window: entry.context_window,
        max_output_tokens: Some(entry.max_output_tokens),
        default_max_tokens: Some(entry.default_max_tokens),
        source: "catalog".into(),
        source_url: Some(entry.source.clone()),
    })
}

pub fn automatic(profile: &Profile) -> Option<Limits> {
    if profile.adapter == "codex" {
        return None;
    }
    profile
        .limits
        .model_limits
        .as_ref()
        .filter(|limits| {
            limits.source == "service"
                && limits.endpoint == profile.endpoint.trim_end_matches('/')
                && model_id(&limits.model) == model_id(&profile.model)
                && (8192..=2_000_000).contains(&limits.context_window)
        })
        .cloned()
        .or_else(|| known(profile))
        .or_else(|| {
            Some(Limits {
                endpoint: profile.endpoint.trim_end_matches('/').into(),
                model: profile.model.clone(),
                context_window: UNKNOWN_CONTEXT,
                max_output_tokens: None,
                default_max_tokens: (profile.adapter == "anthropic-native").then_some(8192),
                source: "fallback".into(),
                source_url: None,
            })
        })
}

pub fn window(profile: &Profile) -> Option<usize> {
    if profile.limits.context_window_mode != Some(Mode::Auto)
        && let Some(window) = profile.context_window
    {
        return Some(window);
    }
    automatic(profile).map(|limits| limits.context_window)
}

pub fn output_cap(profile: &Profile) -> Option<usize> {
    // Native Codex owns its output envelope; this adapter cannot send max_tokens.
    if profile.adapter == "codex" {
        return None;
    }
    profile.limits.max_output_tokens.or_else(|| {
        automatic(profile).and_then(|limits| {
            limits.default_max_tokens.map(|cap| {
                limits
                    .max_output_tokens
                    .map_or(cap, |maximum| cap.min(maximum))
            })
        })
    })
}

/// DSH: min(window * .8, window - reserved output - compaction headroom).
pub fn threshold(profile: &Profile, window: usize) -> Option<usize> {
    let available = window.checked_sub(output_cap(profile).unwrap_or(0))?;
    let pressure = available.checked_sub(
        profile
            .limits
            .compaction_headroom
            .unwrap_or(DEFAULT_HEADROOM),
    )?;
    let threshold = (window * 4 / 5).min(pressure);
    (available * 16 / 100 < threshold).then_some(threshold)
}

pub fn from_service(profile: &Profile, row: &Value) -> Option<Limits> {
    fn number(row: &Value, keys: &[&str]) -> Option<usize> {
        keys.iter().find_map(|key| {
            row[key]
                .as_u64()
                .or_else(|| row[key].as_str()?.parse().ok())
                .and_then(|n| usize::try_from(n).ok())
                .filter(|n| *n > 0)
        })
    }
    let context_window = number(
        row,
        &[
            "contextWindow",
            "context_window",
            "context_length",
            "inputTokenLimit",
            "max_input_tokens",
        ],
    )?;
    if !(8192..=2_000_000).contains(&context_window) {
        return None;
    }
    let maximum = number(
        row,
        &["outputTokenLimit", "maxOutputTokens", "max_output_tokens"],
    );
    let mut limits = known(profile).unwrap_or(Limits {
        endpoint: profile.endpoint.trim_end_matches('/').into(),
        model: profile.model.clone(),
        context_window,
        max_output_tokens: None,
        default_max_tokens: (profile.adapter == "gemini-native").then_some(65_536),
        source: String::new(),
        source_url: None,
    });
    limits.context_window = context_window;
    limits.max_output_tokens = maximum.or(limits.max_output_tokens);
    limits.source = "service".into();
    Some(limits)
}

pub fn apply_automatic(profile: &mut Profile) {
    if profile.limits.context_window_mode == Some(Mode::Auto) || profile.context_window.is_none() {
        profile.limits.context_window_mode = Some(Mode::Auto);
        profile.limits.model_limits = automatic(profile);
        // Materialize for older app versions; mode keeps it distinct from an override.
        profile.context_window = profile
            .limits
            .model_limits
            .as_ref()
            .map(|limits| limits.context_window);
    }
}

#[cfg(test)]
#[path = "context_tests.rs"]
mod tests;
