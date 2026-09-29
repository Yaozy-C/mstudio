use crate::assistant::{harness::session, journal};
use crate::database::Store;
use rig_core::message::Message;
use serde_json::json;

#[test]
fn packed_session_job_and_tool_result_survive_reopen() {
    let root = std::env::temp_dir().join(format!("mstudio-content-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','p','{}',0)", [])
        .unwrap();
    let large = "Long user instructions 图片 ".repeat(1000);
    let message = Message::user(&large);
    let binding = json!({"agentId":"production"});
    session::start(
        &store,
        "p",
        "t",
        binding.clone(),
        std::slice::from_ref(&message),
    )
    .unwrap();
    let job = json!({"id":"j","projectId":"p","input":{"image":large},"status":"COMPLETED"});
    crate::jobs::save(&store, &job).unwrap();
    journal::append(
        &store,
        "p",
        "t",
        "tool/result",
        json!({"callId":"c","value":{"text":large},"message":message}),
    )
    .unwrap();
    journal::append(
        &store,
        "p",
        "images",
        "image/offload",
        json!({"offloads":[{"id":"image-1","image":{"data":large}}]}),
    )
    .unwrap();
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    assert_eq!(crate::jobs::get(&store, "j").unwrap(), job);
    let restored = session::restore(&store, "p", "t", &binding)
        .unwrap()
        .unwrap();
    assert_eq!(
        serde_json::to_value(&restored[0]).unwrap(),
        serde_json::to_value(&message).unwrap()
    );
    let image = super::stored_image::read(&store.db.lock().unwrap(), "p", "image-1");
    assert_eq!(image["__offloadedImage"]["data"], large);
    assert_eq!(
        super::stored_image::read(&store.db.lock().unwrap(), "other", "image-1")["code"],
        "IMAGE_NOT_FOUND"
    );
    let result = super::tool_output::read_page(&store, "p", "t", "c", 0);
    assert!(
        result["text"]
            .as_str()
            .unwrap()
            .contains("Long user instructions")
    );
    assert!(result["nextOffset"].is_number());
    let page = journal::turn_page(&store, "p", "t", None).unwrap();
    assert_eq!(page[0]["payload"]["callId"], "c");
    assert!(page[0]["payload"].get("value").is_none());
    crate::project_storage::remove(&store, "p").unwrap();
    assert_eq!(
        store
            .db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM content_blobs", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
