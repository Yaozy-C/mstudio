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
    let profile = profiles::builtins()
        .into_iter()
        .find(|p| p.id == "editor")
        .unwrap();
    let editor_system = system(
        &json!({"agent":{"name":profile.id,"instructions":profile.instructions,"tools":profile.tool_ids,"skills":profile.skill_ids}}),
    );
    assert!(editor_system.contains("Timeline data:"));
    assert!(!editor_system.contains("Script editing:"));
    assert!(!editor_system.contains("Shot editing:"));
    // editor is now the only timeline role, so its own instructions legitimately
    // appear. The role-isolation this line used to check is preserved by requiring
    // another shipped role's instructions to stay absent.
    assert!(editor_system.contains("Own selection of existing media"));
    assert!(!editor_system.contains("Own every still and video generation task"));
    assert!(editor_system.contains("Use complete savedValues[].values"));
    assert!(editor_system.contains("waiting_user"));
    assert!(editor_system.contains("mstudio_await_generation(taskKeys)"));
    assert!(editor_system.contains("Skills provide domain constraints"));
    assert!(!editor_system.contains("recheck graded pixels and transition composites"));
    let system = system(
        &json!({"agent":{"tools":["project-read","agent-delegate"]},"specialists":[{"id":"concept"}]}),
    );
    assert!(system.contains("concept"));
    assert!(!system.contains("Script editing:"));
}
#[test]
fn generation_rules_also_cover_standalone_asset_tasks() {
    let system =
        system(&json!({"agent":{"tools":["project-read","project-assets","media-generation"]}}));
    assert!(system.contains("Media generation:"));
    assert!(!system.contains("Frame editing:"));
    assert!(!system.contains("Production editing:"));
}
