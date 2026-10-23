use super::*;
#[tokio::test]
#[ignore = "requires local Codex login and consumes one native image generation"]
async fn live_codex_image() {
    let root = std::env::temp_dir().join(format!("mstudio-codex-live-{}", mstudio::media::id()));
    std::fs::create_dir_all(&root).unwrap();
    persist(&root, &update("IN_QUEUE", None, None).unwrap()).unwrap();
    let value = tokio::time::timeout(Duration::from_secs(600), generate(
            &json!({"prompt":"A minimalist blue ceramic cup on a white background. No text.","model":"gpt-image-2","n":1}),
            &root, CancellationToken::new(),
        )).await.expect("timed out").expect("native image generation failed");
    let outputs = CODEX.outputs(&value).unwrap();
    assert_eq!(outputs.len(), 1);
    let (_, bytes) = super::super::image_data::image_bytes(&outputs[0].url).unwrap();
    let path = root.join("result.png");
    std::fs::write(&path, bytes).unwrap();
    println!("Native Codex image saved: {}", path.display());
}
#[test]
fn observed_phases_are_persisted_without_image_payloads() {
    let root = std::env::temp_dir().join(format!("mstudio-codex-phase-{}", mstudio::media::id()));
    std::fs::create_dir_all(&root).unwrap();
    persist(&root, &update("IN_QUEUE", None, None).unwrap()).unwrap();
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
    persist(&directory, &update("IN_PROGRESS", None, None).unwrap()).unwrap();
    let job = json!({"providerConfig":config,"requestId":"test"});
    assert_eq!(read(&job).unwrap().status, "FAILED");
    persist(&directory, &update("IN_QUEUE", None, None).unwrap()).unwrap();
    assert_eq!(read(&job).unwrap().status, "FAILED");
    persist(
        &directory,
        &update(
            "COMPLETED",
            Some(json!({"data":[{"b64_json":"YQ=="}]})),
            None,
        )
        .unwrap(),
    )
    .unwrap();
    assert_eq!(read(&job).unwrap().status, "COMPLETED");
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn restart_preserves_partial_direct_results() {
    let root = std::env::temp_dir().join(format!("mstudio-codex-batch-{}", mstudio::media::id()));
    std::fs::create_dir_all(&root).unwrap();
    let config = json!({"codexRoot":root});
    let directory = directory(&config, "partial").unwrap();
    std::fs::create_dir_all(&directory).unwrap();
    let result = json!({"data":[{"b64_json":"iVBORw0KGgo="}]});
    persist(
        &directory,
        &update("IN_PROGRESS", Some(result.clone()), None).unwrap(),
    )
    .unwrap();
    phase(&directory, "generating", "已生成 1 张图片").unwrap();
    let recovered = read(&json!({"requestId":"partial", "providerConfig":config})).unwrap();
    assert_eq!(recovered.status, "FAILED");
    assert_eq!(recovered.result, Some(result));
    assert_eq!(recovered.outputs.unwrap().len(), 1);
    std::fs::remove_dir_all(root).unwrap();
}
#[tokio::test]
async fn queued_requests_wait_for_capacity_and_can_be_cancelled() {
    let slots = tokio::sync::Semaphore::new(2);
    let cancel = CancellationToken::new();
    let first = wait_for_slot(&slots, &cancel).await.unwrap();
    let second = wait_for_slot(&slots, &cancel).await.unwrap();
    assert!(
        tokio::time::timeout(Duration::from_millis(20), wait_for_slot(&slots, &cancel))
            .await
            .is_err()
    );
    let cancelled = CancellationToken::new();
    cancelled.cancel();
    assert!(wait_for_slot(&slots, &cancelled).await.is_err());
    drop(first);
    let third = wait_for_slot(&slots, &cancel).await.unwrap();
    assert_eq!(slots.available_permits(), 0);
    drop((second, third));
    assert_eq!(slots.available_permits(), 2);
}

#[tokio::test]
#[ignore = "requires Codex login; consumes one generation and one edit"]
async fn live_codex_direct_generation_and_edit() {
    let root = std::env::temp_dir().join(format!("mstudio-codex-direct-{}", mstudio::media::id()));
    std::fs::create_dir_all(&root).unwrap();
    persist(&root, &update("IN_QUEUE", None, None).unwrap()).unwrap();
    let generated = generate(
        &json!({"prompt":"白色背景上的一个蓝色陶瓷杯。","model":"gpt-image-2"}),
        &root,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    let image = CODEX.outputs(&generated).unwrap().remove(0).url;
    let edited = generate(
        &json!({"prompt":"将杯子改成红色，保持白色背景。","model":"gpt-image-2", "image":[image]}),
        &root,
        CancellationToken::new(),
    )
    .await
    .unwrap();
    for (name, result) in [("generated", generated), ("edited", edited)] {
        let outputs = CODEX.outputs(&result).unwrap();
        assert_eq!(outputs.len(), 1);
        let (_, bytes) = super::super::image_data::image_bytes(&outputs[0].url).unwrap();
        std::fs::write(root.join(format!("{name}.png")), bytes).unwrap();
    }
    println!("Direct generation and edit saved: {}", root.display());
}

#[test]
fn corrupted_state_cannot_be_replaced_by_a_progress_or_partial_fallback() {
    let root = std::env::temp_dir().join(format!("codex-broken-{}", mstudio::media::id()));
    std::fs::create_dir_all(&root).unwrap();
    assert!(phase(&root, "generating", "test").is_err());
    std::fs::write(root.join("state.json"), b"broken").unwrap();
    assert!(phase(&root, "generating", "test").is_err());
    assert!(finish(&root, Err(anyhow::anyhow!("request failed")), false).is_err());
    assert!(finish(&root, Err(anyhow::anyhow!("cancelled")), true).is_err());
    assert_eq!(std::fs::read(root.join("state.json")).unwrap(), b"broken");
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn failure_and_cancel_preserve_canonical_partial_batch() {
    let root = std::env::temp_dir().join(format!("codex-partial-{}", mstudio::media::id()));
    std::fs::create_dir_all(&root).unwrap();
    persist(
        &root,
        &update(
            "IN_PROGRESS",
            Some(json!({"data":[{"b64_json":"YQ=="},{"b64_json":"Yg=="}]})),
            None,
        )
        .unwrap(),
    )
    .unwrap();
    for cancelled in [false, true] {
        let result = finish(&root, Err(anyhow::anyhow!("request failed")), cancelled).unwrap();
        let mut job = json!({});
        result.apply(&mut job).unwrap();
        assert_eq!(
            job["status"],
            if cancelled { "CANCELLED" } else { "FAILED" }
        );
        assert_eq!(job["outputCount"], 2);
        assert_eq!(job["outputs"][1]["url"], "data:image/png;base64,Yg==");
    }
    assert!(update("COMPLETED", Some(json!({"data":[]})), None).is_err());
    std::fs::remove_dir_all(root).unwrap();
}
