use super::*;
fn inputs() -> Vec<Input> {
    vec![Input {
        asset_id: "a".into(),
        role: "reference".into(),
        purpose: "原图".into(),
        start: None,
        end: None,
    }]
}
#[test]
fn response_cannot_change_references_or_return_empty_prompt() {
    assert!(
        parse_reply(
            r#"{"prompt":"布面纹理","references":[{"assetId":"a","purpose":"商品材质"}]}"#,
            &inputs()
        )
        .is_ok()
    );
    for raw in [
        r#"{"prompt":"布面","references":[]}"#,
        r#"{"prompt":"布面","references":[{"assetId":"b","purpose":"参考"}]}"#,
        r#"{"prompt":"","references":[{"assetId":"a","purpose":"参考"}]}"#,
    ] {
        assert!(parse_reply(raw, &inputs()).is_err());
    }
}
#[test]
fn explicit_text_context_is_resolved_from_project_without_becoming_media() {
    let (root, store, doc) = super::super::attachment_tests::fixture();
    let request: Request = serde_json::from_value(json!({
        "projectId": "p", "mediaModelId": "image", "kind": "image", "prompt": "根据文字生成", "parameters": {},
        "inputs": [], "contextReferences": [{"kind": "node", "id": "script"}]
    })).unwrap();
    let payload = super::super::attachments::payload(
        &store,
        &doc,
        &request.prompt,
        &request.context_references,
        &super::super::config::Profile::default(),
    )
    .unwrap()
    .to_string();
    assert!(payload.contains("Keep the main character."));
    assert!(!payload.contains("Only change the light."));
    assert!(parse_reply(r#"{"prompt":"人物画面","references":[]}"#, &request.inputs).is_ok());
    let missing = vec![super::super::attachments::Reference {
        kind: "node".into(),
        id: "missing".into(),
    }];
    assert!(
        super::super::attachments::payload(
            &store,
            &doc,
            &request.prompt,
            &missing,
            &super::super::config::Profile::default()
        )
        .is_err()
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn unresolved_spatial_conflict_stops_preparation() {
    let error = parse_reply(
        r#"{"error":"Camera is fixed inside but must also dive in from outside."}"#,
        &[],
    )
    .err()
    .unwrap();
    assert!(error.contains("Camera is fixed inside"));
    assert!(error.contains("尚未提交生成任务"));
    // A mixed success/error reply must not bypass the stop.
    assert!(
        parse_reply(
            r#"{"error":"conflict","prompt":"invented compromise","references":[]}"#,
            &[]
        )
        .is_err()
    );
}
#[test]
fn fenced_json_is_accepted() {
    assert!(
        parse_reply(
            "```json\n{\"prompt\":\"实拍画面\",\"references\":[]}\n```",
            &[]
        )
        .is_ok()
    );
}
#[test]
fn authoring_context_is_selected_by_media_kind() {
    let image = rules::system("image");
    assert!(image.contains("IMAGE AUTHORING:"));
    assert!(!image.contains("VIDEO AUTHORING:"));
    assert!(!image.contains("Preserve the requested events, dialogue, duration"));
    let video = rules::system("video");
    assert!(video.contains("VIDEO AUTHORING:"));
    assert!(!video.contains("IMAGE AUTHORING:"));
    for context in [image, video] {
        assert!(context.contains("preserve its order, IDs and count exactly"));
        assert!(context.contains("specific conflict and the decision needed"));
    }
}
