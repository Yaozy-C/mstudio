use super::{attachment_tests::fixture, context, history};
use crate::database::Store;
use serde_json::{Value, json};

fn request(project: &str) -> Value {
    json!({"refs":[{"kind":"asset","id":"image","title":"产品图","assetId":"image","mediaKind":"image"}],
        "agentId":"concept","targetNodeId":"script","selectedNodeId":"script","selection":{"clipId":null,"time":0},
        "production":{"projectId":project,"models":{"image":"image-model"},"instruction":"根据图片写脚本"}})
}
fn begin(store: &Store, turn: &str) -> anyhow::Result<()> {
    let request = request("p");
    history::begin(
        store,
        "p",
        turn,
        &request,
        "model",
        &json!({"agentId":"concept","agentName":"创意策划","turnId":turn,"request":request}),
    )
}

#[test]
fn accepted_request_survives_process_exit_with_refs_and_retry_context() {
    let (root, store, _) = fixture();
    begin(&store, "crash-turn").unwrap();
    let before = history::read(&store, "p").unwrap();
    assert_eq!(before.len(), 2);
    assert_eq!(before[0].content, "根据图片写脚本");
    assert_eq!(before[1].attribution.as_ref().unwrap()["status"], "running");
    let ids = (before[0].id, before[1].id);
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    let after = history::read(&store, "p").unwrap();
    assert_eq!((after[0].id, after[1].id), ids);
    assert_eq!(after[0].payload[0]["attachments"][0]["id"], "image");
    assert_eq!(
        after[0].attribution.as_ref().unwrap()["request"],
        request("p")
    );
    assert_eq!(
        after[1].attribution.as_ref().unwrap()["status"],
        "interrupted"
    );
    assert!(after[1].content.is_empty());
    assert!(
        !context::assemble(&after, json!("next"), json!({}))
            .unwrap()
            .to_string()
            .contains("根据图片写脚本")
    );
    assert!(history::read(&store, "other").unwrap().is_empty());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn success_failure_and_cancellation_are_durable_and_repeated_delivery_cannot_duplicate_them() {
    let (root, store, _) = fixture();
    for (turn, status, text, error) in [
        ("success", "completed", "成功的脚本", None),
        ("failure", "failed", "", Some("模型回答超时")),
        ("cancel", "cancelled", "", Some("已停止")),
    ] {
        begin(&store, turn).unwrap();
        assert!(begin(&store, turn).is_err());
        history::finish(&store, "p", turn, text, status, error).unwrap();
        assert!(history::finish(&store, "p", turn, text, status, error).is_err());
    }
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    let saved = history::read(&store, "p").unwrap();
    assert_eq!(saved.len(), 6);
    assert_eq!(saved[1].content, "成功的脚本");
    assert_eq!(
        saved[3].attribution.as_ref().unwrap()["error"],
        "模型回答超时"
    );
    assert_eq!(
        saved[5].attribution.as_ref().unwrap()["status"],
        "cancelled"
    );
    let input = context::assemble(&saved, json!("next"), json!({}))
        .unwrap()
        .to_string();
    assert!(input.contains("成功的脚本"));
    assert!(!input.contains("模型回答超时"));
    assert_eq!(input.matches("根据图片写脚本").count(), 1);
    assert!(begin(&store, "failure").is_err());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn attachment_enrichment_and_deleted_project_cannot_erase_or_resurrect_sent_messages() {
    let (root, store, _) = fixture();
    begin(&store, "media").unwrap();
    let full = json!([{"type":"text","text":"引用资料","attachments":request("p")["refs"]},
        {"type":"image_url","image_url":{"url":"data:image/png;base64,secret-bytes"}}]);
    history::update_payload(&store, "p", "media", &context::text_only(&full)).unwrap();
    let messages = history::read(&store, "p").unwrap();
    assert_eq!(messages[0].content, "根据图片写脚本");
    assert!(!messages[0].payload.to_string().contains("secret-bytes"));
    store
        .db
        .lock()
        .unwrap()
        .execute("DELETE FROM projects WHERE id='p'", [])
        .unwrap();
    assert!(history::finish(&store, "p", "media", "late answer", "completed", None).is_err());
    assert!(begin(&store, "another-turn").is_err());
    assert!(history::read(&store, "p").unwrap().is_empty());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
