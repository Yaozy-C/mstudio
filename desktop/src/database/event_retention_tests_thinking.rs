use super::*;

#[test]
fn readable_thinking_survives_turn_completion_but_stream_snapshots_are_transient() {
    let store = store();
    begin(&store, "thinking");
    for text in ["先", "先检查"] {
        journal::append(
            &store,
            "p",
            "thinking",
            "assistant/thinking",
            json!({"id":"a","text":text}),
        )
        .unwrap();
    }
    journal::append(
        &store,
        "p",
        "thinking",
        "assistant/reasoning",
        json!({"id":"a","text":"先检查"}),
    )
    .unwrap();
    journal::append(
        &store,
        "p",
        "thinking",
        "assistant/progress",
        json!({"text":"核对参考"}),
    )
    .unwrap();
    journal::append(
        &store,
        "p",
        "thinking",
        "turn/end",
        json!({"status":"completed"}),
    )
    .unwrap();
    let page = journal::turn_page(&store, "p", "thinking", None).unwrap();
    assert!(
        !page
            .iter()
            .any(|event| event["kind"] == "assistant/thinking")
    );
    assert!(page.iter().any(
        |event| event["kind"] == "assistant/reasoning" && event["payload"]["text"] == "先检查"
    ));
    assert!(
        page.iter()
            .any(|event| event["kind"] == "assistant/progress")
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
