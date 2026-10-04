use super::{harness::schema, profiles, tool_schema};
use serde_json::{Value, json};

fn issues(role: &str, operation: Value) -> Vec<schema::Issue> {
    let agent = profiles::builtins()
        .into_iter()
        .find(|p| p.id == role)
        .unwrap();
    schema::issues(
        &tool_schema::for_profile(&agent),
        &json!({"action":"edit","operations":[operation]}),
    )
}

#[test]
fn generation_contract_exposes_only_executable_role_variants() {
    let image = json!({"op":"request_generation","mediaKind":"image","text":"Synthetic image"});
    assert!(issues("storyboard-artist", image.clone()).is_empty());
    assert!(!issues("asset-designer", image.clone()).is_empty());
    let mut asset = image.clone();
    asset["generationPurpose"] = json!("asset");
    asset["references"] = json!([]);
    assert!(issues("asset-designer", asset.clone()).is_empty());
    for field in ["id", "canvasTaskKey", "mode", "modelId"] {
        let mut invalid = asset.clone();
        invalid[field] = json!("value");
        assert!(
            issues("asset-designer", invalid)
                .iter()
                .any(|i| i.path == format!("$.operations[0].{field}"))
        );
    }
    let mut video = image.clone();
    video["mediaKind"] = json!("video");
    video["mode"] = json!("ends");
    assert!(issues("production", video.clone()).is_empty());
    assert!(!issues("storyboard-artist", video.clone()).is_empty());
    video["mode"] = json!("image");
    assert!(
        issues("production", video)
            .iter()
            .any(|i| i.path.ends_with(".mode"))
    );
    let mut invalid = image;
    invalid["references"] = json!([{"assetId":"ref","purpose":"identity","role":"first-frame"}]);
    assert!(
        issues("storyboard-artist", invalid)
            .iter()
            .any(|i| i.path.ends_with(".role"))
    );
}

#[test]
fn missing_operation_reports_all_locatable_errors_without_rewriting_input() {
    let input = json!({"type":"request_generation","mediaKind":"image","mode":"image","modelId":"selected","text":"Synthetic"});
    let errors = issues("asset-designer", input.clone());
    let paths: Vec<_> = errors.iter().map(|i| i.path.as_str()).collect();
    for field in [
        "op",
        "type",
        "mode",
        "modelId",
        "generationPurpose",
        "references",
    ] {
        assert!(
            paths.contains(&format!("$.operations[0].{field}").as_str()),
            "{errors:?}"
        );
    }
    assert!(input.get("op").is_none());
    let unknown = issues("storyboard-artist", json!({"op":"invented"}));
    assert_eq!(unknown[0].path, "$.operations[0].op");
    assert!(unknown[0].message.contains("request_generation"));
}

#[test]
fn descriptive_reference_roles_explain_the_purpose_field() {
    let errors = issues(
        "asset-designer",
        json!({
            "op":"request_generation", "mediaKind":"image", "generationPurpose":"asset",
            "text":"Synthetic image", "references":[
                {"assetId":"ref", "purpose":"reference", "role":"product geometry, top view"}
            ]
        }),
    );
    let role = errors
        .iter()
        .find(|issue| issue.path.ends_with(".role"))
        .unwrap();
    assert!(role.message.contains("purpose"));
    assert!(role.message.contains("reference"));
}

#[test]
fn advertised_reference_descriptions_distinguish_image_and_video_modes() {
    let contract = tool_schema::schema();
    let operations = contract["properties"]["operations"]["items"]["oneOf"]
        .as_array()
        .unwrap();
    for operation in operations
        .iter()
        .filter(|op| op["properties"]["op"]["const"] == "request_generation")
    {
        let input = &operation["properties"]["references"]["items"];
        let role = input["properties"]["role"]["description"].as_str().unwrap();
        assert!(
            input["description"]
                .as_str()
                .unwrap()
                .contains("\"purpose\":\"Preserve product geometry")
        );
        assert!(
            input["properties"]["purpose"]["description"]
                .as_str()
                .unwrap()
                .contains("never in role")
        );
        if operation["properties"]["mediaKind"]["const"] == "image" {
            assert!(role.contains("edit = source image"));
            assert!(!role.contains("video-reference"));
        } else {
            assert!(role.contains("first-frame = starting image"));
            assert!(role.contains("video-reference for videos"));
            assert!(!role.contains("edit ="));
        }
        assert!(role.contains("purpose"));
    }
}

#[test]
fn production_fields_distinguish_draft_prompt_task_prompt_and_video_settings() {
    assert!(
        issues(
            "production",
            json!({"op":"update_node","id":"shot","shot":{"prompt":"Draft"}})
        )
        .is_empty()
    );
    assert!(
        issues(
            "production",
            json!({"op":"update_node","id":"shot","prompt":"Draft"})
        )
        .iter()
        .any(|i| i.path == "$.operations[0].prompt")
    );
    let valid = json!({"op":"request_generation","id":"shot","mediaKind":"video","text":"Generate this action","mode":"multi","references":[{"assetId":"image","role":"reference","purpose":"Identity"}],"parameters":{"duration":5,"resolution":"1080P","aspectRatio":"9:16"}});
    assert!(issues("production", valid.clone()).is_empty());
    let mut invalid = valid;
    invalid["mode"] = json!("reference");
    invalid["parameters"]["duration"] = json!(4);
    invalid["parameters"]["resolution"] = json!("1080p");
    let errors = issues("production", invalid);
    for field in ["mode", "parameters.duration", "parameters.resolution"] {
        assert!(
            errors
                .iter()
                .any(|e| e.path == format!("$.operations[0].{field}")),
            "{errors:?}"
        );
    }
    assert!(
        errors
            .iter()
            .any(|e| e.path.ends_with(".mode") && e.message.contains("mode=multi"))
    );
}

#[test]
fn image_and_video_advertise_only_their_parameter_fields() {
    for (kind, fields) in [
        ("image", vec!["duration"]),
        ("video", vec!["width", "height"]),
    ] {
        for field in fields {
            let mut value =
                json!({"op":"request_generation","mediaKind":kind,"text":"Prompt","parameters":{}});
            value["parameters"][field] = json!(16);
            assert!(
                issues("production", value)
                    .iter()
                    .any(|e| e.path == format!("$.operations[0].parameters.{field}"))
            );
        }
    }
    let image = json!({"op":"request_generation","mediaKind":"image","text":"Prompt","parameters":{"resolution":"1080P"}});
    assert!(
        issues("production", image)
            .iter()
            .any(|e| e.path.ends_with(".resolution"))
    );
    let video = json!({"op":"request_generation","mediaKind":"video","text":"Prompt","parameters":{"resolution":"1K"}});
    assert!(
        issues("production", video)
            .iter()
            .any(|e| e.path.ends_with(".resolution"))
    );
}
