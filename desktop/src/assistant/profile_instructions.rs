//! Upgrade recognized defaults; preserve custom instructions and capability choices.
use super::profiles::AgentProfile;
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::collections::BTreeMap;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Baseline {
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    variants: Vec<Baseline>,
    instructions: Vec<String>,
    description: String,
    skill_ids: Vec<String>,
    tool_ids: Vec<String>,
}
fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
pub(super) fn upgrade(saved: &mut [AgentProfile], builtins: &[AgentProfile]) -> bool {
    let baseline = serde_json::from_str(include_str!("profile_instruction_baseline.json"))
        .expect("valid shipped profile baseline");
    apply(saved, builtins, &baseline)
}
fn apply(
    saved: &mut [AgentProfile],
    builtins: &[AgentProfile],
    baseline: &BTreeMap<String, Baseline>,
) -> bool {
    let mut changed = false;
    for profile in saved {
        let Some(old) = baseline.get(&profile.id) else {
            continue;
        };
        let Some(current) = builtins.iter().find(|p| p.id == profile.id) else {
            continue;
        };
        let hash = digest(&profile.instructions);
        let Some(old) = std::iter::once(old)
            .chain(old.variants.iter())
            .find(|version| version.instructions.contains(&hash))
        else {
            continue;
        };
        let name =
            old.name.as_deref() == Some(profile.name.as_str()) && profile.name != current.name;
        if name {
            profile.name.clone_from(&current.name);
        }
        let instructions = profile.instructions != current.instructions;
        let description =
            profile.description == old.description && profile.description != current.description;
        let skills = profile.skill_ids == old.skill_ids && profile.skill_ids != current.skill_ids;
        let tools = profile.tool_ids == old.tool_ids && profile.tool_ids != current.tool_ids;
        if instructions {
            profile.instructions.clone_from(&current.instructions);
        }
        if description {
            profile.description.clone_from(&current.description);
        }
        // Upgrade untouched defaults; custom capability choices remain intact.
        if skills {
            profile.skill_ids.clone_from(&current.skill_ids);
        }
        if tools {
            profile.tool_ids.clone_from(&current.tool_ids);
        }
        if name || instructions || description || skills || tools {
            profile.revision += 1;
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::profiles;
    #[test]
    fn upgrade_preserves_custom_profiles_and_restrictions_and_is_idempotent() {
        let defaults = profiles::builtins();
        let mut saved = vec![
            defaults[0].clone(),
            defaults[0].clone(),
            defaults[0].clone(),
        ];
        for p in &mut saved {
            p.instructions = "previous shipped instructions".into();
            p.skill_ids = vec!["ad-script".into()];
            p.tool_ids = vec!["project-read".into(), "agent-delegate".into()];
        }
        saved[0].name = "My assistant".into();
        saved[0].enabled = false;
        saved[1].tool_ids = vec!["project-read".into()];
        saved[2].instructions = "User authored".into();
        let custom = serde_json::to_value(&saved[2]).unwrap();
        let baseline = BTreeMap::from([(
            "coordinator".into(),
            Baseline {
                name: None,
                variants: vec![],
                instructions: vec![digest("previous shipped instructions")],
                description: saved[0].description.clone(),
                skill_ids: saved[0].skill_ids.clone(),
                tool_ids: saved[0].tool_ids.clone(),
            },
        )]);
        assert!(apply(&mut saved, &defaults, &baseline));
        assert_eq!(saved[0].tool_ids, defaults[0].tool_ids);
        assert_eq!(saved[0].skill_ids, defaults[0].skill_ids);
        assert_eq!(saved[0].name, "My assistant");
        assert!(!saved[0].enabled);
        assert_eq!(saved[1].tool_ids, ["project-read"]);
        assert_eq!(serde_json::to_value(&saved[2]).unwrap(), custom);
        assert!(!apply(&mut saved, &defaults, &baseline));
    }
    #[test]
    fn previous_combined_default_splits_without_granting_creative_script_writes() {
        let defaults = profiles::builtins();
        let mut old = defaults.iter().find(|p| p.id == "concept").unwrap().clone();
        old.instructions = r#"Own content ideas, viewing payoff, product relationship and the audiovisual screenplay. Read ad-script and only methods relevant to the current writing task. Write concrete events, exact dialogue, visible copy and meaningful sound; preserve the current market, language, duration and user-selected direction. Use project facts, supplied references and imported research; Mstudio has no network research tools. Save structured screenplay paragraphs when requested, preserving existing IDs. Do not create or edit shot records, camera plans, model prompts, media tasks or timelines. Handoff the chosen script and essential reveal/order constraints to the shot director; camera parameters and execution belong downstream. A script does not authorize generation."#.into();
        old.name = "创意编剧".into();
        old.description = "创意方向、观看回报与声画脚本".into();
        old.skill_ids = vec!["ad-script".into()];
        old.tool_ids = vec![
            "project-read".into(),
            "project-script".into(),
            "memory-read".into(),
        ];
        old.enabled = false;
        let mut saved = vec![old];
        assert!(upgrade(&mut saved, &defaults));
        assert_eq!(saved[0].name, "创意策划");
        assert_eq!(saved[0].skill_ids, ["creative-concepts"]);
        assert!(!saved[0].enabled);
        assert!(!profiles::allows(&saved[0], "edit"));
        assert!(!upgrade(&mut saved, &defaults));
        let writer = defaults.iter().find(|p| p.id == "writer").unwrap();
        assert!(crate::assistant::permissions::allows_operation(
            writer, "add_node"
        ));
    }
}
