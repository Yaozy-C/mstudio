use super::*;
use crate::assistant::{harness::agent_artifacts, journal};

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
    let doc = json!({"revision":8,"production":{"drafts":{key:{
        "key":key,"turnId":"video-turn","createdAt":1,
        "kind":"video","prompt":"保持连续镜头和原台词",
        "inputs":[{"assetId":"board","role":"reference"}],
        "parameters":{"duration":6}
    }}}});
    let refs = json!([{"kind":"generation","id":key}]);
    let resolved = resolve(&store, "p", &doc, Some(&refs)).unwrap();
    assert!(resolved.attachments.is_empty());
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
