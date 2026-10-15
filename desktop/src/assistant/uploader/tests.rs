use super::*;
use serde_json::json;

pub(super) fn body() -> Value {
    json!({"input":[{"role":"user","content":[{"type":"input_image","image_url":"data:image/png;base64,aGVsbG8="}]}]})
}

#[test]
fn projection_only_changes_images_and_preserves_all_three_protocols() {
    let mut value = json!({
        "tools":[{"parameters":{"image_url":"data:image/png;base64,Yg=="}}],
        "input":[{"content":[{"type":"input_image","image_url":"data:image/png;base64,YQ=="}]},
            {"type":"function_call_output","call_id":"read","output":[{"type":"input_image","image_url":"data:image/png;base64,YQ=="}]}],
        "messages":[{"role":"user","content":[{"type":"image_url","image_url":{"url":"data:image/png;base64,YQ=="}},
            {"type":"image","source":{"type":"base64","media_type":"image/png","data":"YQ=="}},
            {"type":"image_url","image_url":{"url":"https://example.test/p.png"}}]}]
    });
    let images = projection::collect(&value).unwrap();
    assert_eq!(images.len(), 4);
    assert!(
        images
            .iter()
            .all(|(_, image)| image.variant == images[0].1.variant)
    );
    for (pointer, _) in images {
        backends::deepseek::DeepSeek
            .project(value.pointer_mut(&pointer).unwrap(), "file-api-test")
            .unwrap();
    }
    assert_eq!(
        value["input"][0]["content"][0],
        json!({"type":"input_image","file_id":"file-api-test"})
    );
    assert_eq!(value["input"][1]["call_id"], "read");
    assert_eq!(
        value["messages"][0]["content"][0],
        json!({"type":"file","file_id":"file-api-test"})
    );
    assert_eq!(
        value["messages"][0]["content"][1]["source"],
        json!({"type":"file","file_id":"file-api-test"})
    );
    assert!(value["messages"][0]["content"][2]["image_url"].is_object());
    assert!(
        value["tools"][0]["parameters"]["image_url"]
            .as_str()
            .unwrap()
            .contains("Yg==")
    );
    assert!(projection::collect(&value).unwrap().is_empty());
}

#[test]
fn index_is_durable_scoped_by_credentials_and_refreshes_before_expiry() {
    let root = std::env::temp_dir().join(format!("mstudio-files-index-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    let files = Uploader::new(
        Arc::new(backends::deepseek::DeepSeek),
        "https://api.deepseek.com",
        "test-key",
        Some(&store),
    )
    .unwrap();
    let image = projection::collect(&body()).unwrap().remove(0).1;
    let record = Uploaded {
        id: "file-api-test".into(),
        bytes: 5,
        expires_at: now() + LIFETIME,
    };
    index::put(
        &files.db.lock().unwrap(),
        &files.scope,
        &image.variant,
        &record,
    )
    .unwrap();
    drop(files);
    let files = Uploader::new(
        Arc::new(backends::deepseek::DeepSeek),
        "https://api.deepseek.com/",
        "test-key",
        Some(&store),
    )
    .unwrap();
    assert_eq!(
        index::get(
            &files.db.lock().unwrap(),
            &files.scope,
            &image.variant,
            now()
        )
        .unwrap()
        .as_deref(),
        Some("file-api-test")
    );
    for (endpoint, key) in [
        ("https://api.deepseek.com", "another-key"),
        ("https://other.test", "test-key"),
    ] {
        let other = Uploader::new(
            Arc::new(backends::deepseek::DeepSeek),
            endpoint,
            key,
            Some(&store),
        )
        .unwrap();
        assert!(
            index::get(
                &other.db.lock().unwrap(),
                &other.scope,
                &image.variant,
                now()
            )
            .unwrap()
            .is_none()
        );
    }
    assert!(
        index::get(
            &files.db.lock().unwrap(),
            &files.scope,
            &image.variant,
            now() + LIFETIME - REFRESH_MARGIN
        )
        .unwrap()
        .is_none()
    );
    // An older rejected generation cannot remove a newer mapping.
    index::remove(
        &files.db.lock().unwrap(),
        &files.scope,
        &image.variant,
        "file-api-old",
    )
    .unwrap();
    assert!(
        index::get(
            &files.db.lock().unwrap(),
            &files.scope,
            &image.variant,
            now()
        )
        .unwrap()
        .is_some()
    );
    let schema: String = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT sql FROM sqlite_master WHERE name='provider_uploads'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert!(!schema.contains("key"));
    drop(files);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn unrelated_image_failures_are_not_treated_as_stale_ids() {
    let files = Uploader::new(
        Arc::new(backends::deepseek::DeepSeek),
        "https://api.deepseek.com",
        "test-key",
        None,
    )
    .unwrap();
    let used = vec![Used {
        variant: "hash".into(),
        id: "file-api-test".into(),
    }];
    for (status, message) in [
        (400, "invalid image file format"),
        (401, "file_id expired"),
        (429, "file quota exceeded"),
        (500, "file_id invalid"),
    ] {
        let failure = Error::non_success_with_details(
            status.try_into().unwrap(),
            Default::default(),
            json!({"error":{"message":message}}).to_string(),
        );
        assert!(!files.invalidate_stale(&failure, &used).unwrap());
    }
}
