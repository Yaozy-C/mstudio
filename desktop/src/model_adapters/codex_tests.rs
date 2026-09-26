use super::*;
#[tokio::test]
#[ignore = "requires local Codex login and consumes one native image generation"]
async fn live_codex_image() {
    let root = std::env::temp_dir().join(format!("mstudio-codex-live-{}", mstudio::media::id()));
    std::fs::create_dir_all(&root).unwrap();
    let value = tokio::time::timeout(Duration::from_secs(600), generate(
            &json!({"prompt":"A minimalist blue ceramic cup on a white background. No text.","model":"codex-image","n":1}),
            &root, CancellationToken::new(),
        )).await.expect("timed out").expect("native image generation failed");
    let outputs = CODEX.outputs(&value);
    assert_eq!(outputs.len(), 1);
    let (_, bytes) = super::super::image_data::image_bytes(&outputs[0].url).unwrap();
    let path = root.join("result.png");
    std::fs::write(&path, bytes).unwrap();
    println!("Native Codex image saved: {}", path.display());
}
#[test]
fn codex_model_survives_service_migration_without_api_credentials() {
    let root = std::env::temp_dir().join(format!("mstudio-codex-model-{}", mstudio::media::id()));
    let store = crate::database::Store::open(root.clone()).unwrap();
    let models = json!([{"id":"codex", "name":"Codex image", "kind":"image", "plugin":"codex-image", "endpoint":"codex://local/images", "params":{"model":"codex-image","n":1},"enabled":true}]);
    store
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO settings VALUES('media-models',?1)",
            [models.to_string()],
        )
        .unwrap();
    drop(store);
    let store = crate::database::Store::open(root.clone()).unwrap();
    let rows = crate::models::media::resolved(&store.db.lock().unwrap()).unwrap();
    assert_eq!(rows.len(), 1);
    assert!(rows[0].connection_id.is_none());
    crate::models::media::validate(&rows[0]).unwrap();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn observed_phases_are_persisted_without_image_payloads() {
    let root = std::env::temp_dir().join(format!("mstudio-codex-phase-{}", mstudio::media::id()));
    std::fs::create_dir_all(&root).unwrap();
    phase(&root, "generating", "Codex 正在生成图片").unwrap();
    let state: Value =
        serde_json::from_slice(&std::fs::read(root.join("state.json")).unwrap()).unwrap();
    assert_eq!(state["status"], "IN_PROGRESS");
    assert_eq!(state["progress"]["stage"], "generating");
    assert!(state.get("result").is_none());
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn restarted_job_becomes_failed_without_resubmission() {
    let root = std::env::temp_dir().join(format!("mstudio-codex-state-{}", mstudio::media::id()));
    let config = json!({"codexRoot":root});
    let directory = directory(&config, "test").unwrap();
    std::fs::create_dir_all(&directory).unwrap();
    persist(&directory, &update("IN_PROGRESS", None, None)).unwrap();
    let job = json!({"providerConfig":config,"requestId":"test"});
    assert_eq!(read(&job).unwrap().status, "FAILED");
    persist(
        &directory,
        &update("COMPLETED", Some(json!({"data":[]})), None),
    )
    .unwrap();
    assert_eq!(read(&job).unwrap().status, "COMPLETED");
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn multi_image_prompt_and_restart_preserve_partial_results() {
    let root = std::env::temp_dir().join(format!("mstudio-codex-batch-{}", mstudio::media::id()));
    std::fs::create_dir_all(&root).unwrap();
    let input = json!({"prompt":"FRONT, RIGHT, TOP","n":3});
    let items = codex_request::turn_input(&input, &root).unwrap();
    let prompt = items[0]["text"].as_str().unwrap();
    assert!(prompt.contains("as many times as needed"));
    assert!(!prompt.contains("exactly"));
    let config = json!({"codexRoot":root});
    let directory = directory(&config, "partial").unwrap();
    std::fs::create_dir_all(&directory).unwrap();
    let result = json!({"data":[{"b64_json":"iVBORw0KGgo="}]});
    persist(
        &directory,
        &update("IN_PROGRESS", Some(result.clone()), None),
    )
    .unwrap();
    phase(&directory, "generating", "已生成 1 张图片").unwrap();
    let recovered = read(&json!({"requestId":"partial", "providerConfig":config})).unwrap();
    assert_eq!(recovered.status, "FAILED");
    assert_eq!(recovered.result, Some(result));
    assert_eq!(recovered.outputs.unwrap().len(), 1);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn tool_failure_does_not_interrupt_later_image_results() {
    let events = [
        json!({"type":"imageGeneration","status":"completed","result":"iVBORw0KGgo="}),
        json!({"type":"imageGeneration","status":"failed","failure":{"message":"temporary failure"}}),
        json!({"type":"imageGeneration","status":"completed","result":"iVBORw0KGgo="}),
    ];
    let mut images = Vec::new();
    for event in events {
        if let Some(image) = codex_request::image_result(&event).unwrap() {
            images.push(image);
        }
    }
    assert_eq!(CODEX.outputs(&json!({"data":images})).len(), 2);
}
