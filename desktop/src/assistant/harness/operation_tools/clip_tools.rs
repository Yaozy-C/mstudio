use super::{OperationTool, add, flat};
use serde_json::{Value, json};
pub(super) fn add_updates(out: &mut Vec<OperationTool>, operation: &Value) {
    let op = "update_clip";
    for (name, fields, description) in [
        (
            "trim_clip",
            vec!["trimIn", "trimOut"],
            "Set source in/out seconds on a clip; preserves output placement and speed.",
        ),
        (
            "set_clip_transform",
            vec!["x", "y", "scale", "opacity"],
            "Set a clip's picture placement, scale or opacity.",
        ),
        (
            "set_clip_audio",
            vec!["volume", "fadeIn", "fadeOut"],
            "Set a clip's volume and audio fade durations in seconds.",
        ),
    ] {
        let mut parameters = operation.clone();
        parameters["properties"]
            .as_object_mut()
            .unwrap()
            .retain(|key, _| key == "op" || key == "id" || fields.contains(&key.as_str()));
        add(
            out,
            name,
            op,
            parameters,
            Default::default(),
            None,
            description,
        );
    }
    add(
        out,
        "set_clip_visual",
        op,
        flat(
            operation,
            "visual",
            &[
                "brightness",
                "contrast",
                "saturation",
                "temperature",
                "effect",
            ],
        ),
        Default::default(),
        Some("visual"),
        "Set basic clip brightness, contrast, saturation, temperature or effect using direct fields.",
    );
    let mut grade = operation.clone();
    grade["properties"]["grade"] = operation["properties"]["visual"]["properties"]["grade"].clone();
    add(
        out,
        "set_clip_grade",
        op,
        flat(&grade, "grade", &[]),
        Default::default(),
        Some("visual.grade"),
        "Set clip grading using direct exposure, tonal, HSL, curves or wheel fields. Omitted grade fields are preserved.",
    );
    for (name, visual, description) in [
        (
            "clear_clip_grade",
            json!({"grade":null}),
            "Clear only custom grading on a clip.",
        ),
        (
            "clear_clip_visual",
            Value::Null,
            "Clear all visual adjustments on a clip.",
        ),
    ] {
        add(
            out,
            name,
            op,
            json!({"type":"object","properties":{"id":operation["properties"]["id"]},"required":["id"],"additionalProperties":false}),
            serde_json::Map::from_iter([("visual".into(), visual)]),
            None,
            description,
        );
    }
}
