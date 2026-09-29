//! Model-facing catalog labels are independent of localized UI labels.
use serde_json::{Value, json};

pub(super) fn model_metadata(mut item: Value) -> Value {
    let (name, description) = match item["id"].as_str().unwrap_or_default() {
        "image-prompt" => (
            "Image prompts",
            "Visible composition, poses, support and reference translation",
        ),
        "video-prompt" => (
            "Video prompts",
            "Action, timing, contact and sound translation",
        ),
        "asset-preparation" => (
            "Reference assets",
            "White-background character, wardrobe, product and prop references",
        ),
        "color-grading" => (
            "Color grading",
            "Color correction, shot matching and supported tool boundaries",
        ),
        "transition-design" => (
            "Transition design",
            "Action matching, pacing and shot joins",
        ),
        "ad-script" => (
            "Creative audiovisual script",
            "Concepts, visible events, on-screen text and sound",
        ),
        "storyboard-art" => (
            "Storyboard frames",
            "Still composition, image prompts and local image repairs",
        ),
        "product-storyboard" => (
            "Shot design",
            "Video intent, story, action beats and corresponding shots",
        ),
        "creative-ad-director" => (
            "Creative direction",
            "Advertising concepts, cinematography and model prompt conversion",
        ),
        "product-video-production" => (
            "Video production",
            "References, segmented production, local repair and final review",
        ),
        "ad-team" => (
            "Team coordination",
            "Specialist collaboration, handoffs and local revisions",
        ),
        _ => return item,
    };
    item["name"] = json!(name);
    item["description"] = json!(description);
    item
}
pub(super) fn model_catalog(value: Value) -> Value {
    match value {
        Value::Array(items) => Value::Array(items.into_iter().map(model_metadata).collect()),
        other => other,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn model_catalog_is_english_without_mutating_ui_catalog_or_rule_bodies() {
        let ui = json!([{"id":"ad-team","name":"团队统筹","description":"专业协作","core":"CUSTOM_USER_RULE","revision":42}]);
        let model = model_catalog(ui.clone());
        assert_eq!(ui[0]["name"], "团队统筹");
        assert_eq!(model[0]["name"], "Team coordination");
        assert_eq!(model[0]["core"], "CUSTOM_USER_RULE");
        assert_eq!(model[0]["revision"], 42);
    }
}
