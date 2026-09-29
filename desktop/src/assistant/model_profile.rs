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
            "Creative scriptwriter",
            "Concepts, advertising events and audiovisual scripts",
        ),
        "storyboard" => (
            "Storyboard director",
            "Viewpoint, staging, camera, action beats and continuity",
        ),
        "asset-designer" => (
            "Asset designer",
            "Inventory and prepare missing shared white-background image references",
        ),
        "storyboard-artist" => (
            "Storyboard artist",
            "Shot composition, image generation and local image repair",
        ),
        "production" => (
            "Media producer",
            "Model prompts, references, production and local repair",
        ),
        "editor" => (
            "Picture and sound editor",
            "Take selection, multitrack editing, captions and sound timing",
        ),
        "reviewer" => (
            "Independent reviewer",
            "Read-only verification of facts, actions, continuity and expression",
        ),
        "colorist" => ("Colorist", "Color correction, shot matching and grading"),
        "transition-designer" => (
            "Transition designer",
            "Action/composition matching, joins and pacing",
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
