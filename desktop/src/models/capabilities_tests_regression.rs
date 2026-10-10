use super::*;

#[test]
fn native_reference_declarations_match_the_wire_builder() {
    for (plugin, key) in [("gemini-native", "/image_urls"), ("codex-image", "/image")] {
        let mut declared: Capabilities = serde_json::from_value(json!({"references":[{
            "key":key,"kind":"image","role":"reference","multiple":true
        }]}))
        .unwrap();
        validate(&model(plugin, "unused", Some(declared.clone()))).unwrap();
        declared.references.as_mut().unwrap()[0].key = "/ignored".into();
        assert!(validate(&model(plugin, "unused", Some(declared))).is_err());
    }
    for pointer in ["/bad~2", "/__proto__/polluted", "/constructor/prototype/x"] {
        assert!(!super::super::validation::pointer_is_valid(pointer));
    }
}

#[test]
fn submission_enforces_array_shape_without_implicit_count_caps() {
    let m = model(
        "fal",
        "vendor/model",
        Some(
            serde_json::from_value(json!({
                "references":[{"key":"/images","kind":"image","role":"reference","multiple":true}]
            }))
            .unwrap(),
        ),
    );
    validate_input(&m, &json!({"images":vec!["a";20]})).unwrap();
    for body in [json!({"images":"a"}), json!({"images":[{}]})] {
        assert!(validate_input(&m, &body).is_err());
    }
    let m = model(
        "fal",
        "vendor/model",
        Some(
            serde_json::from_value(json!({
                "references":[{"key":"/image","kind":"image","role":"first-frame"}]
            }))
            .unwrap(),
        ),
    );
    assert!(validate_input(&m, &json!({"image":["a"]})).is_err());
}

#[test]
fn numeric_bounds_and_total_limits_are_configured_even_on_known_endpoints() {
    let capabilities: Capabilities = serde_json::from_value(json!({
        "controls":{"duration":{"path":"/input/seconds","min":0.25,"max":120.5}},
        "referenceLimit":30,"referenceSeconds":180.5,
        "references":[{"key":"/images","kind":"image","role":"reference","multiple":true}]
    }))
    .unwrap();
    let m = model("fal", "minimax/h3/image-to-video", Some(capabilities));
    validate(&m).unwrap();
    assert_eq!(limits(&m).references, Some(30));
    assert_eq!(limits(&m).seconds, Some(180.5));
    validate_input(&m, &json!({"images":vec!["ref";30]})).unwrap();
    assert!(validate_input(&m, &json!({"images":vec!["ref";31]})).is_err());
}
