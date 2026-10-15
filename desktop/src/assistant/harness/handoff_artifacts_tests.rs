use super::*;
use crate::assistant::{harness::agent_artifacts, journal};

#[test]
fn delegation_preserves_the_persisted_user_instruction() {
    let (root, store, doc) = crate::assistant::attachment_tests::fixture();
    let model = crate::assistant::attachment_tests::multimodal();
    let role = crate::assistant::profiles::builtins().remove(0);
    let source = json!({
        "senderAgentId":"coordinator", "senderTurnId":"parent",
        "originalRequest":{"production":{"instruction":"生成镜头吧"},"refs":[]},
        "artifacts":[], "refs":[], "expectedGenerations":[]
    });
    let message = message(&store, &doc, &model, &role, "导演设计五个镜头", &source).unwrap();
    let text = serde_json::to_string(&message).unwrap();
    assert!(text.contains("Original user request (provided directly by the host): 生成镜头吧"));
    assert!(text.contains("导演设计五个镜头"));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn artifact_images_stay_references_while_explicit_attachments_keep_pixels() {
    use rig_core::message::{Message, UserContent};
    let (root, store, doc) = crate::assistant::attachment_tests::fixture();
    let model = crate::assistant::attachment_tests::multimodal();
    let role = crate::assistant::profiles::builtins()
        .into_iter()
        .find(|p| p.id == "director")
        .unwrap();
    let refs = json!([{"kind":"asset","id":"image"}]);
    let resolved = resolve(&store, "p", &doc, Some(&refs)).unwrap();
    let mut source = json!({
        "senderAgentId":"coordinator", "senderTurnId":"parent",
        "originalRequest":{"production":{"instruction":"设计镜头"},"refs":[]},
        "artifacts":resolved.manifest, "refs":refs, "expectedGenerations":[]
    });
    // Saved artifact references remain usable even when their pixels are unavailable.
    std::fs::remove_file(root.join("assets/image.jpg")).unwrap();
    let reference = message(&store, &doc, &model, &role, "设计镜头", &source).unwrap();
    let text = serde_json::to_string(&reference).unwrap();
    assert!(text.contains("mstudio_read_assets") && text.contains("image"));
    let Message::User { content } = reference else {
        panic!("Expected user message")
    };
    assert!(
        !content
            .iter()
            .any(|part| matches!(part, UserContent::Image(_)))
    );
    // The user's explicit attachment still supplies actual pixels, without duplicates.
    std::fs::write(root.join("assets/image.jpg"), b"test-source-content").unwrap();
    source["originalRequest"]["refs"] = refs;
    let explicit = message(&store, &doc, &model, &role, "设计镜头", &source).unwrap();
    let Message::User { content } = explicit else {
        panic!("Expected user message")
    };
    assert_eq!(
        content
            .iter()
            .filter(|part| matches!(part, UserContent::Image(_)))
            .count(),
        1
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn references_preserve_exact_output_across_pages_reopen_and_later_turns() {
    let root = std::env::temp_dir().join(format!("mstudio-artifacts-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','p','{}',0)", [])
        .unwrap();
    let output = "不要改变提手。\n".repeat(2000);
    let artifact = json!({"childId":"child","status":"failed","stopReason":"max-tokens","output":output,"changes":[]});
    journal::append(&store, "p", "first", "subagent/settled", artifact.clone()).unwrap();
    journal::append(
        &store,
        "p",
        "second",
        "subagent/settled",
        json!({"childId":"child","output":"later"}),
    )
    .unwrap();
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    let mut offset = 0;
    let mut text = String::new();
    loop {
        let page = agent_artifacts::read_page(
            &store,
            "p",
            &json!({"id":"child","turnId":"first","offset":offset}),
        );
        assert_eq!(page["stopReason"], "max-tokens");
        text.push_str(page["text"].as_str().unwrap());
        let Some(next) = page["nextOffset"].as_u64() else {
            break;
        };
        offset = next;
    }
    assert_eq!(serde_json::from_str::<Value>(&text).unwrap(), artifact);
    let reference = json!([agent_artifacts::reference("child", "first")]);
    let resolved = resolve(&store, "p", &json!({}), Some(&reference)).unwrap();
    assert_eq!(resolved.manifest[0]["status"], "failed");
    assert!(resolve(&store, "other", &json!({}), Some(&reference)).is_err());
    assert_eq!(
        agent_artifacts::read_page(&store, "p", &json!({"id":"child","turnId":"missing"}))["code"],
        "AGENT_RESULT_NOT_FOUND"
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn object_references_are_validated_without_accepting_relay_content() {
    let root = std::env::temp_dir().join(format!("mstudio-artifacts-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    let doc = json!({"revision":7,"nodes":[{"id":"n","kind":"shot"}],"assets":[{"id":"a"}]});
    let refs = json!([{"kind":"node","id":"n"},{"kind":"asset","id":"a"}]);
    let result = resolve(&store, "p", &doc, Some(&refs)).unwrap();
    assert_eq!(result.attachments.len(), 2);
    assert_eq!(result.manifest[0]["readWith"]["tool"], "mstudio_read_shots");
    for bad in [
        json!([{"kind":"asset","id":"missing"}]),
        json!([{"kind":"asset","id":"a","prompt":"invented size"}]),
        json!([{"kind":"agent","id":"child"}]),
    ] {
        assert!(resolve(&store, "p", &doc, Some(&bad)).is_err());
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn video_task_handoff_uses_saved_task_key_without_accepting_rewritten_inputs() {
    let root = std::env::temp_dir().join(format!("mstudio-artifacts-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    let key = "run:video:exec-1:0";
    let doc = json!({"revision":8,"assets":[{"id":"result","kind":"video"},{"id":"discarded","discarded":true}],"production":{"drafts":{key:{
        "key":key,"turnId":"video-turn","createdAt":1,
        "kind":"video","prompt":"保持连续镜头和原台词",
        "inputs":[{"assetId":"board","role":"reference"}],
        "parameters":{"duration":6},"resultAssetId":"result","resultAssetIds":["result","discarded","missing"]
    }}}});
    let refs = json!([{"kind":"generation","id":key}]);
    let resolved = resolve(&store, "p", &doc, Some(&refs)).unwrap();
    assert_eq!(resolved.attachments.len(), 1);
    assert_eq!(resolved.attachments[0].id, "result");
    assert!(resolved.manifest[0]["savedTask"].get("prompt").is_none());
    assert_eq!(
        resolved.manifest[0]["savedTask"]["parameters"]["duration"],
        6
    );
    assert!(
        !source_message("使用视频", "parent", "turn", &resolved.manifest)
            .contains("保持连续镜头和原台词")
    );
    assert_eq!(resolved.manifest[0]["projectRevision"], 8);
    assert_eq!(
        resolved.manifest[0]["readWith"],
        json!({
            "tool":"mstudio_read_generation","arguments":{"taskKey":key}
        })
    );
    assert!(resolve(&store, "p", &json!({}), Some(&refs)).is_err());
    let mut unsubmitted = doc.clone();
    unsubmitted["production"]["drafts"][key]["createdAt"] = Value::Null;
    assert!(resolve(&store, "p", &unsubmitted, Some(&refs)).is_err());
    for field in ["prompt", "inputs", "parameters"] {
        let mut rewritten = refs.clone();
        rewritten[0][field] = json!("relay replacement");
        assert!(resolve(&store, "p", &doc, Some(&rewritten)).is_err());
    }
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn followup_receives_reference_ids_without_saved_video_prompt() {
    let root = std::env::temp_dir().join(format!("handoff-full-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    let prompt = "complete saved motion ".repeat(300);
    let doc = json!({"revision":9,"nodes":[{"id":"s3","kind":"shot","shot":{"prompt":prompt},"references":[{"assetId":"board","purpose":"sequence"}]}]});
    let result = resolve(&store, "p", &doc, Some(&json!([{"kind":"node","id":"s3"}]))).unwrap();
    assert!(result.manifest[0].get("savedVideoPrompt").is_none());
    assert!(
        !crate::assistant::creative_context::node_context(&doc, &doc["nodes"][0])
            .to_string()
            .contains(&prompt)
    );
    assert!(!source_message("修正图片", "parent", "turn", &result.manifest).contains(&prompt));
    assert_eq!(result.manifest[0]["savedReferences"][0]["assetId"], "board");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
