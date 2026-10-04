use super::*;
use crate::database::{Store, blobs};
#[test]
fn original_prompt_is_resolved_by_output_identity_with_no_media_or_secret_loading() {
    let (store, mut doc, profile) = super::super::tests::fixture();
    doc["assets"] = json!([{"id":"image","name":"renamed.png","kind":"image"},{"id":"import","name":"generated-job.png","kind":"image"}]);
    crate::projects::write_document(&store, doc, false).unwrap();
    let prompt = "原始提交文字".repeat(1000);
    let mut job = json!({"id":"job","asset":{"id":"image"},"assets":[{"id":"image"}],"mediaModelId":"model","input":{"prompt":prompt,"image":"DO_NOT_LOAD","duration":7,"api_key":"DO_NOT_EXPOSE"},"providerConfig":{"authorization":"SECRET"},"shot":{"canvasGeneration":{"task":{"key":"actual-task","prompt":"later draft"}},"references":[{"assetId":"ref","role":"reference","purpose":"identity"}]}});
    {
        let db = store.db.lock().unwrap();
        db.execute("INSERT INTO jobs VALUES('job','p','{}')", [])
            .unwrap();
        blobs::pack(&db, blobs::Owner::Job("job"), &mut job).unwrap();
        db.execute("UPDATE jobs SET data=?1 WHERE id='job'", [job.to_string()])
            .unwrap();
    }
    let inspect = |ids: Value, offset: usize| {
        super::super::execute(&store,&profile,"p","turn","read",json!({"action":"inspect","section":"assets","ids":ids,"fields":["source"],"textOffset":offset})).unwrap()
    };
    let first = inspect(json!(["image"]), 0);
    assert_eq!(first["items"][0]["source"]["jobId"], "job");
    assert_eq!(first["items"][0]["source"]["taskKey"], "actual-task");
    let mut text = first["items"][0]["source"]["prompt"]["text"]
        .as_str()
        .unwrap()
        .to_owned();
    let second = inspect(json!(["image"]), 4000);
    text.push_str(
        second["items"][0]["source"]["prompt"]["text"]
            .as_str()
            .unwrap(),
    );
    assert_eq!(text, prompt);
    assert!(second["items"][0]["source"]["prompt"]["nextTextOffset"].is_null());
    assert!(!first.to_string().contains("SECRET"));
    assert!(!first.to_string().contains("DO_NOT"));
    assert!(!first.to_string().contains("later draft"));
    assert_eq!(
        inspect(json!(["import"]), 0)["items"][0]["source"]["status"],
        "unavailable"
    );
    assert!(
        inspect(json!(["missing"]), 0)["items"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(
        source(&store.db.lock().unwrap(), "other", "image", true, 0, 4000).unwrap()["status"],
        "unavailable"
    );
    cleanup(store);
}
#[test]
fn batched_sources_obey_page_budget_and_report_ambiguous_provenance() {
    let (store, mut doc, profile) = super::super::tests::fixture();
    doc["assets"] = json!(
        (0..12)
            .map(|i| json!({"id":format!("a{i}"),"name":"renamed","kind":"image"}))
            .collect::<Vec<_>>()
    );
    crate::projects::write_document(&store, doc, false).unwrap();
    {
        let db = store.db.lock().unwrap();
        for i in 0..12 {
            db.execute(
                "INSERT INTO jobs VALUES(?1,'p',?2)",
                params![
                    format!("j{i}"),
                    json!({"assets":[{"id":format!("a{i}")}],"input":{"prompt":"文".repeat(2000)}})
                        .to_string()
                ],
            )
            .unwrap();
        }
        db.execute(
            "INSERT INTO jobs VALUES('duplicate','p',?1)",
            [json!({"asset":{"id":"a0"}}).to_string()],
        )
        .unwrap();
    }
    let mut offset = 0;
    let mut ids = Vec::new();
    loop {
        let page = super::super::execute(
            &store,
            &profile,
            "p",
            "turn",
            "read",
            json!({"action":"inspect","section":"assets","fields":["source"],"offset":offset}),
        )
        .unwrap();
        assert!(page.to_string().len() < 12500);
        for item in page["items"].as_array().unwrap() {
            ids.push(item["id"].clone());
        }
        if let Some(next) = page["nextOffset"].as_u64() {
            assert!(next > offset);
            offset = next;
        } else {
            break;
        }
    }
    assert_eq!(ids.len(), 12);
    assert_eq!(
        source(&store.db.lock().unwrap(), "p", "a0", true, 0, 1000).unwrap()["status"],
        "ambiguous"
    );
    cleanup(store);
}
fn cleanup(store: Store) {
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
