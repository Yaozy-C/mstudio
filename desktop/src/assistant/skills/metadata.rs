//! Model-facing catalog labels are independent of localized UI labels.
use serde_json::{Value, json};

pub(super) fn model_metadata(mut item: Value) -> Value {
    let (name, description) = match item["id"].as_str().unwrap_or_default() {
        "creative-concepts" => (
            "Creative concepts",
            "Viewing motives, core events, payoff, product relationship and direction comparison",
        ),
        "ad-script" => (
            "Audiovisual screenwriting",
            "Realize a selected concept as action, dialogue, visible text, sound and segment timing",
        ),
        "creative-ad-director" => (
            "Creative direction",
            "Viewing proposition, shot design, on-camera performance, photographic appearance and rhythm",
        ),
        "image-production" => (
            "Image production",
            "Image prompts, storyboard frames, reference assets and frame inspection",
        ),
        "product-video-production" => (
            "Video production",
            "Video prompts, input purposes, generation, repair and result judgement",
        ),
        "video-editing" => (
            "Editing and finishing",
            "Source selection, pace, colour, transitions and sound",
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
        let ui = json!([{"id":"ad-script","name":"创意与声画脚本","description":"专业协作","core":"CUSTOM_USER_RULE","revision":42}]);
        let model = model_catalog(ui.clone());
        assert_eq!(ui[0]["name"], "创意与声画脚本");
        assert_eq!(model[0]["name"], "Audiovisual screenwriting");
        assert_eq!(model[0]["core"], "CUSTOM_USER_RULE");
        assert_eq!(model[0]["revision"], 42);
    }
}
