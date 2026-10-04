use super::*;
use crate::assistant::{profiles, tool_schema};
use serde_json::json;

fn han(text: &str) -> bool {
    text.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c))
}
#[test]
fn builtin_instructions_and_tool_contracts_are_english_but_content_language_is_preserved() {
    assert!(!han(&tool_schema::schema().to_string()));
    for profile in profiles::builtins() {
        assert!(!han(&profile.instructions), "{}", profile.id);
        let system = system(
            &json!({"agent":{"name":profile.id,"instructions":profile.instructions,"tools":profile.tool_ids,"skills":profile.skill_ids}}),
        );
        assert!(!han(&system), "{}", profile.id);
        assert!(system.contains("Reply in the user's language"));
        assert!(system.contains("Preserve the requested language of scripts"));
    }
}
#[test]
fn permissions_supply_data_semantics_without_assigning_unrelated_professional_roles() {
    for role in ["colorist", "transition-designer"] {
        let profile = profiles::builtins()
            .into_iter()
            .find(|p| p.id == role)
            .unwrap();
        let system = system(
            &json!({"agent":{"name":profile.id,"instructions":profile.instructions,"tools":profile.tool_ids,"skills":profile.skill_ids}}),
        );
        assert!(system.contains("Timeline data:"));
        assert!(!system.contains("Script editing:"));
        assert!(!system.contains("Shot editing:"));
        assert!(!system.contains("Own selection of existing media"));
        assert!(system.contains("Verify complete savedValues[].values"));
        assert!(system.contains("waiting_user"));
        assert!(system.contains("mstudio_await_generation(taskKeys)"));
        assert!(system.contains("recheck graded pixels and transition composites"));
    }
    let system = system(
        &json!({"agent":{"tools":["project-read","agent-delegate"]},"specialists":[{"id":"concept"}]}),
    );
    assert!(system.contains("concept"));
    assert!(!system.contains("Script editing:"));
    assert!(!system.contains("Project memory:"));
}
#[test]
fn generation_rules_also_cover_standalone_asset_tasks() {
    let system =
        system(&json!({"agent":{"tools":["project-read","project-assets","media-generation"]}}));
    assert!(system.contains("Media generation:"));
    assert!(!system.contains("Frame editing:"));
    assert!(!system.contains("Production editing:"));
}
