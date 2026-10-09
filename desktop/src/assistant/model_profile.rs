//! English built-in role metadata for model requests; UI and custom labels are preserved.
use super::profiles::{AgentProfile, allows, builtins};
use serde_json::{Value, json};

fn labels(profile: &AgentProfile) -> (String, String) {
    let Some(builtin) = builtins().into_iter().find(|p| p.id == profile.id) else {
        return (profile.name.clone(), profile.description.clone());
    };
    let (name, description) = match profile.id.as_str() {
        "coordinator" => (
            "Project coordinator",
            "Goals, facts, scope, dependencies and specialist handoffs",
        ),
        "concept" => (
            "Creative planner",
            "Concept directions, viewing motives, core events and product relationship",
        ),
        "writer" => (
            "Screenwriter",
            "Audiovisual screenplay, dialogue, text, sound and segment timing",
        ),
        "director" => (
            "Shot director",
            "Camera, staging, performance, timing and shot design",
        ),
        "image" => (
            "Image production",
            "Still prompts, reference assets, image generation and repair",
        ),
        "production" => (
            "Video production",
            "Video prompts, control inputs, generation and repair",
        ),
        "editor" => (
            "Editing and finishing",
            "Source selection, pace, colour, transitions, captions and sound",
        ),
        _ => return (profile.name.clone(), profile.description.clone()),
    };
    (
        if profile.name == builtin.name {
            name.into()
        } else {
            profile.name.clone()
        },
        if profile.description == builtin.description {
            description.into()
        } else {
            profile.description.clone()
        },
    )
}
pub(super) fn role(profile: &AgentProfile) -> Value {
    let (name, _) = labels(profile);
    json!({"name":name,"instructions":profile.instructions,"skills":profile.skill_ids,"tools":profile.tool_ids,"canEdit":allows(profile,"edit")})
}
pub(super) fn summary(profile: &AgentProfile) -> Value {
    let (name, description) = labels(profile);
    json!({"id":profile.id,"name":name,"description":description,"tools":profile.tool_ids})
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_metadata_does_not_change_ui_labels_or_custom_text() {
        let mut profile = builtins().remove(0);
        assert_eq!(role(&profile)["name"], "Project coordinator");
        assert_eq!(profile.name, "项目统筹");
        profile.name = "我的团队".into();
        profile.description = "用户定制职责".into();
        assert_eq!(role(&profile)["name"], "我的团队");
        assert_eq!(summary(&profile)["description"], "用户定制职责");
    }
}
