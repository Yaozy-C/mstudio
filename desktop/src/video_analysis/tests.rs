use super::*;

#[test]
fn operations_are_exclusive_cancellable_and_reusable_without_a_database() {
    let runtime = Runtime::default();
    let id = uuid::Uuid::new_v4().to_string();
    assert!(runtime.begin("../outside").is_err());
    let operation = runtime.begin(&id).unwrap();
    assert!(runtime.begin(&id).is_err());
    runtime.active.lock().unwrap().get(&id).unwrap().cancel();
    assert!(operation.token.is_cancelled());
    drop(operation);
    assert!(runtime.begin(&id).is_ok());
    assert!(runtime.sessions.lock().unwrap().is_empty());
}

#[test]
fn temporary_video_is_removed_only_after_the_last_reader_releases_it() {
    let directory = std::env::temp_dir().join(format!("analysis-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let path = directory.join("original.mp4");
    std::fs::write(&path, b"test").unwrap();
    let session = Arc::new(Session {
        directory: directory.clone(),
        video: Video {
            id: "test".into(),
            name: "test".into(),
            path: path.to_string_lossy().into(),
            duration: 2.,
            width: 320,
            height: 240,
            has_audio: true,
        },
    });
    let running = session.clone();
    drop(session);
    assert!(path.is_file());
    drop(running);
    assert!(!directory.exists());
}

#[tokio::test]
async fn model_copy_preserves_duration_and_audio_without_using_partial_outputs() {
    let directory = std::env::temp_dir().join(format!("analysis-test-{}", uuid::Uuid::new_v4()));
    std::fs::create_dir_all(&directory).unwrap();
    let original = directory.join("original.mkv");
    let output = mstudio::media::command("ffmpeg")
        .args([
            "-v",
            "error",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=320x240:rate=24:duration=2",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=2",
            "-c:v",
            "libx264",
            "-c:a",
            "aac",
        ])
        .arg(&original)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let session = Session {
        directory: directory.clone(),
        video: Video {
            id: "test".into(),
            name: "test".into(),
            path: original.to_string_lossy().into(),
            duration: 2.,
            width: 320,
            height: 240,
            has_audio: true,
        },
    };
    std::fs::write(directory.join("analysis-pending.mp4"), b"incomplete output").unwrap();
    let events = Arc::new(Mutex::new(Vec::<serde_json::Value>::new()));
    let received = events.clone();
    let channel = tauri::ipc::Channel::new(move |body| {
        received.lock().unwrap().push(body.deserialize().unwrap());
        Ok(())
    });
    let path = media::model_copy(
        &session,
        &CancellationToken::new(),
        &progress::Reporter(Some(channel)),
    )
    .await
    .unwrap();
    let events = events.lock().unwrap();
    assert!(!events.is_empty());
    assert!(
        events
            .iter()
            .all(|e| e["stage"] == "encoding" && e["total"] == 2_000_000)
    );
    assert!(
        events
            .iter()
            .any(|e| e["completed"].as_u64().unwrap_or(0) > 0)
    );
    let probe = mstudio::media::probe(&path).unwrap();
    let duration: f64 = probe["format"]["duration"]
        .as_str()
        .unwrap()
        .parse()
        .unwrap();
    assert!((duration - 2.).abs() < 0.2);
    assert!(
        probe["streams"]
            .as_array()
            .unwrap()
            .iter()
            .any(|s| s["codec_type"] == "audio")
    );
    assert!(original.is_file());
    assert!(!directory.join("analysis-pending.mp4").exists());
    assert!(std::fs::metadata(path).unwrap().len() < 12 * 1024 * 1024);
}
