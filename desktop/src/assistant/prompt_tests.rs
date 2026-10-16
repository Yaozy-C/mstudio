use super::*;
use crate::assistant::{profiles, tool_schema};
use serde_json::json;

fn han(text: &str) -> bool {
    text.chars().any(|c| ('\u{3400}'..='\u{9fff}').contains(&c))
}
#[test]
fn coordinator_prompt_size() {
    let profile = profiles::builtins().remove(0);
    let prompt = system(&json!({"agent":super::super::model_profile::role(&profile),"skills":[]}));
    let tools = serde_json::to_string(
        &[
            vec![super::super::harness::delegation::definition()],
            super::super::harness::delegation::control_definitions(),
        ]
        .concat(),
    )
    .unwrap();
    println!(
        "coordinator system chars: {}; delegation tool chars: {}",
        prompt.chars().count(),
        tools.chars().count()
    );
    assert!(prompt.chars().count() < 2400);
    assert!(!prompt.contains("Available Skills:"));
    assert!(!prompt.contains("Storyboard handoff contract:"));
    assert!(!prompt.contains("Generation execution contract:"));
    assert!(!prompt.contains("runInBackground")); // Delegation has no foreground override.
}
#[test]
fn only_delegating_roles_receive_dispatch_guidance() {
    let by_id = |id: &str| {
        profiles::builtins()
            .into_iter()
            .find(|p| p.id == id)
            .unwrap()
    };
    let coordinator = system(
        &json!({"agent":{"name":"coordinator","instructions":by_id("coordinator").instructions,"tools":by_id("coordinator").tool_ids,"skills":by_id("coordinator").skill_ids}}),
    );
    assert!(
        coordinator.contains("Delegate once the goal and targets are known without probing first")
    );
    assert!(!coordinator.contains("mstudio_delegate itself must be loaded"));
    let writer = system(
        &json!({"agent":{"name":"writer","instructions":by_id("writer").instructions,"tools":by_id("writer").tool_ids,"skills":by_id("writer").skill_ids}}),
    );
    assert!(!writer.contains("mstudio_delegate"));
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
        if profile.id != "video-analyst" {
            assert!(system.contains("Preserve the requested language of scripts"));
        }
    }
}
#[test]
fn professional_methods_are_not_repeated_in_host_policy() {
    for profile in profiles::builtins() {
        let text =
            system(&json!({"agent":super::super::model_profile::role(&profile),"skills":[]}));
        assert!(text.contains(&profile.instructions));
        for duplicated in [
            "Script editing:",
            "Shot editing:",
            "Frame editing:",
            "Production editing:",
            "Loading is per run",
        ] {
            assert!(!text.contains(duplicated));
        }
        assert!(text.chars().count() < 3000, "{}", profile.id);
    }
}
