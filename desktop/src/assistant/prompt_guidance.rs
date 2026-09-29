//! Deterministic rule loading for prompt authors in chat, delegation and quick generation.
use super::profiles::AgentProfile;
use anyhow::Result;
use rusqlite::Connection;
use serde_json::{Value, json};

#[derive(serde::Deserialize)]
struct Binding {
    kind: String,
    resources: Vec<String>,
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

// Skill assignment is the only trigger. Tool permissions never imply creative capabilities.
fn creative(db: &Connection, profile: &AgentProfile) -> Result<(String, Vec<String>)> {
    let config = config();
    let setting = super::profiles::skill_setting(profile);
    let mut text = String::new();
    let mut kinds = Vec::new();
    let mut loaded = std::collections::HashSet::new();
    for id in &profile.skill_ids {
        let Some(binding) = config.skills.get(id) else {
            continue;
        };
        if !kinds.contains(&binding.kind) {
            kinds.push(binding.kind.clone());
        }
        for path in &binding.resources {
            let mut offset = 0;
            loop {
                let page = super::skills::storage::read(db, &setting, id, path, offset, true)?;
                if offset == 0 {
                    let key = format!("{} / {}", page["skill"], page["path"]);
                    if !loaded.insert(key.clone()) {
                        break;
                    }
                    text.push_str(&format!(
                        "\nAssigned Skill rule (full current text): {key}\n"
                    ));
                }
                text.push_str(page["text"].as_str().unwrap_or_default());
                match page["nextOffset"].as_u64() {
                    Some(next) => offset = next as usize,
                    None => break,
                }
            }
            text.push('\n');
        }
    }
    Ok((text, kinds))
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
    let (mut result, kinds) = creative(db, profile)?;
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
mod tests {
    use super::*;
    use crate::database::Store;
    #[test]
    fn prompt_authors_get_full_current_rules_and_only_their_selected_model() {
        let root =
            std::env::temp_dir().join(format!("mstudio-prompt-rules-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        let db = store.db.lock().unwrap();
        super::super::skills::storage::seed(
            &db,
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills"),
        )
        .unwrap();
        let animation = format!("{}\nCURRENT_ANIMATION_TAIL", "x".repeat(9000));
        db.execute(
            "UPDATE skill_resources SET text=?1 WHERE path='references/animation-principles.md'",
            [&animation],
        )
        .unwrap();
        let models = json!([
            {"id":"h3","name":"Selected video","plugin":"fal","kind":"video","endpoint":"minimax/h3/reference-to-video","enabled":true,"params":{}},
            {"id":"image","name":"Image","plugin":"codex-image","kind":"image","endpoint":"codex://local/images","enabled":true,"params":{}},
            {"id":"other","name":"Other video","plugin":"fal","kind":"video","endpoint":"other/video","enabled":true,"params":{}}
        ]);
        db.execute(
            "INSERT INTO settings VALUES('media-models',?1)",
            [models.to_string()],
        )
        .unwrap();
        let agents = super::super::profiles::builtins();
        let context = json!({"models":{"video":"h3","image":"image"}});
        for id in ["storyboard-artist", "asset-designer", "production"] {
            let profile = agents.iter().find(|a| a.id == id).unwrap();
            let result = for_agent(&db, profile, &context, &json!({})).unwrap();
            assert_eq!(result.matches("CURRENT_ANIMATION_TAIL").count(), 1);
            assert_eq!(result.contains("subject_definitions"), id == "production");
            assert_eq!(
                result.contains("configured Codex image model"),
                id != "production"
            );
        }
        let production = agents.iter().find(|a| a.id == "production").unwrap();
        let switched = for_agent(
            &db,
            production,
            &json!({"models":{"video":"other"}}),
            &json!({}),
        )
        .unwrap();
        assert!(!switched.contains("subject_definitions"));
        let referenced = for_agent(&db, production, &json!({"models":{"video":"other"},"referencedTask":{"key":"run","kind":"video","modelId":"other"}}), &json!({"production":{"drafts":{"run":{"kind":"video","modelId":"h3"}}}})).unwrap();
        assert!(referenced.contains("subject_definitions"));
        for id in [
            "coordinator",
            "concept",
            "storyboard",
            "editor",
            "reviewer",
            "colorist",
            "transition-designer",
        ] {
            let agent = agents.iter().find(|a| a.id == id).unwrap();
            assert!(
                for_agent(&db, agent, &context, &json!({}))
                    .unwrap()
                    .is_empty(),
                "{id}"
            );
        }
        let mut unassigned = production.clone();
        unassigned.skill_ids.retain(|s| s != "video-prompt");
        assert!(
            for_agent(&db, &unassigned, &context, &json!({}))
                .unwrap()
                .is_empty()
        );
        assert_eq!(quick_profile(&db, "image").unwrap().id, "storyboard-artist");
        assert_eq!(quick_profile(&db, "video").unwrap().id, "production");
        let mut configured = agents.clone();
        configured
            .iter_mut()
            .find(|a| a.id == "production")
            .unwrap()
            .skill_ids
            .retain(|s| s != "video-prompt");
        db.execute(
            "UPDATE settings SET value=?1 WHERE key='agents'",
            [serde_json::to_string(&configured).unwrap()],
        )
        .unwrap();
        assert!(quick_profile(&db, "video").is_err());
        drop(db);
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
