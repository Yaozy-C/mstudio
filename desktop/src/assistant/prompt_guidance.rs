//! Selected-model capabilities plus the creative rules a task actually needs.
//!
//! Chat and delegated turns read Skill bodies on demand with tools, so they only
//! receive the selected model's rules here. The generation panel is a single
//! completion with no tool loop, so it must receive the current rule bodies
//! directly: the bound Skill entry, its core, and the method documents its task
//! signals require.
use super::profiles::AgentProfile;
use super::prompt_routing::routes;
use anyhow::Result;
use rusqlite::Connection;
use serde_json::{Value, json};
use std::collections::BTreeSet;

#[derive(serde::Deserialize)]
struct Binding {
    kind: String,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase")]
struct Config {
    skills: std::collections::BTreeMap<String, Binding>,
    quick_agents: std::collections::BTreeMap<String, String>,
}
fn config() -> Config {
    serde_json::from_str(include_str!("skills/prompt_bindings.json"))
        .expect("valid prompt Skill configuration")
}

pub fn quick_profile(db: &Connection, kind: &str) -> Result<AgentProfile> {
    let config = config();
    let id = config
        .quick_agents
        .get(kind)
        .ok_or_else(|| anyhow::anyhow!("没有配置此生成模式的 Agent"))?;
    let profile = super::profiles::resolve(db, Some(id))?;
    anyhow::ensure!(
        profile
            .skill_ids
            .iter()
            .any(|id| config.skills.get(id).is_some_and(|s| s.kind == kind)),
        "该 Agent 未装配对应的提示词 Skill"
    );
    Ok(profile)
}

pub fn for_agent(
    db: &Connection,
    profile: &AgentProfile,
    production: &Value,
    doc: &Value,
) -> Result<String> {
    if profile.id == "video-analyst" {
        return super::video_analysis::guidance(db, profile);
    }
    let config = config();
    let kinds: std::collections::BTreeSet<_> = profile
        .skill_ids
        .iter()
        .filter_map(|id| config.skills.get(id).map(|binding| binding.kind.as_str()))
        .collect();
    let mut result = String::new();
    if kinds.is_empty() {
        return Ok(result);
    }
    let models = crate::models::media::read(db)?;
    for kind in kinds {
        // A referenced task retains its own model even after the composer switches.
        let task_key = production["referencedTask"]["key"].as_str();
        let referenced = task_key
            .and_then(|k| doc["production"]["drafts"].get(k))
            .unwrap_or(&production["referencedTask"]);
        let selected = [referenced, &production["task"]]
            .into_iter()
            .find_map(|t| {
                (t["kind"] == kind)
                    .then(|| t["modelId"].as_str())
                    .flatten()
                    .filter(|s| !s.is_empty())
            })
            .or_else(|| production["models"][&kind].as_str());
        if let Some(id) = selected {
            if let Some(model) = models
                .iter()
                .find(|m| m.id == id && m.enabled && m.kind == kind)
            {
                let rules = crate::model_adapters::prompt_rules::guidance(db, model)?;
                result.push_str(&format!("\nSelected model guidance (applies only to this model; do not copy to another):\n{rules}\n"));
            } else {
                result.push_str(&format!("\nSelected {kind} model {id} is unavailable; resolve selection before generation.\n"));
            }
        }
    }
    Ok(result)
}

/// Load the current rule text a quick generation panel needs, in the database's
/// own words, followed by the selected model's rules.
pub fn for_quick(
    db: &Connection,
    profile: &AgentProfile,
    kind: &str,
    description: &str,
    production: &Value,
    doc: &Value,
) -> Result<String> {
    let config = config();
    let Some(bound) = profile
        .skill_ids
        .iter()
        .find(|id| config.skills.get(*id).is_some_and(|s| s.kind == kind))
    else {
        return Ok(String::new());
    };
    let setting = super::profiles::skill_setting(profile);
    let mut loader = Loader {
        db,
        setting: &setting,
        owner: bound.clone(),
        seen: BTreeSet::new(),
        text: String::new(),
    };
    // The entry document is required: without it the panel would silently lose
    // the rules it is supposed to apply.
    loader.add(bound, "SKILL.md")?;
    loader.add(bound, "CORE.md")?;
    for (skill, path) in routes(kind, description, bound) {
        loader.add(skill, path)?;
    }
    let mut result = format!(
        "\nCurrent creative rules from the project database. Apply them as written; they are not suggestions.\n{}",
        loader.text
    );
    result.push_str(&for_agent(db, profile, production, doc)?);
    Ok(result)
}

struct Loader<'a> {
    db: &'a Connection,
    setting: &'a str,
    owner: String,
    seen: BTreeSet<String>,
    text: String,
}

impl Loader<'_> {
    fn add(&mut self, skill: &str, path: &str) -> Result<()> {
        if !self.seen.insert(format!("{skill}/{path}")) {
            return Ok(());
        }
        anyhow::ensure!(skill == self.owner, "Prompt method outside bound Skill");
        let page = super::skills::storage::read(self.db, self.setting, &self.owner, path, 0, true)?;
        let text = page["text"].as_str().unwrap_or_default();
        anyhow::ensure!(
            !text.trim().is_empty(),
            "Required prompt method is empty: {skill}/{path}"
        );
        self.text
            .push_str(&format!("\n### {skill}/{path}\n{text}\n"));
        Ok(())
    }
}

pub fn attach(
    snapshot: &mut Value,
    db: &Connection,
    profile: &AgentProfile,
    production: &Value,
    doc: &Value,
) -> Result<()> {
    snapshot["promptGuidance"] = json!(for_agent(db, profile, production, doc)?);
    Ok(())
}

#[cfg(test)]
#[path = "prompt_guidance_tests.rs"]
mod tests;
