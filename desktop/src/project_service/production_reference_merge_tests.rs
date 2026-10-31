use super::{generation_prepare, tests::fixture};
use serde_json::json;

#[test]
fn native_generation_maps_short_video_sources_to_one_uploaded_reference() {
    let (store, mut doc, _) = fixture();
    doc["assets"] = json!([
        {"id":"one","kind":"video","usage":"asset","name":"One","duration":1.333333,"width":160,"height":120},
        {"id":"two","kind":"video","usage":"asset","name":"Two","duration":1.333333,"width":160,"height":120}
    ]);
    doc["production"] = json!({"drafts":{"task":{
        "key":"task","title":"Merged reference video","position":{"x":0,"y":0},"kind":"video","mode":"mixed",
        "prompt":"Keep the action order","status":"READY","inputs":[
            {"key":"one","assetId":"one","role":"video-reference","purpose":"Opening"},
            {"key":"two","assetId":"two","role":"video-reference","purpose":"Reveal"}
        ]
    }}});
    let mut request = json!({"action":"prepare_generation","document":doc,"taskKey":"task","model":{
        "id":"model","name":"Video","plugin":"fal","endpoint":"vendor/video",
        "kind":"video","enabled":true,"params":{},"capabilities":{
            "references":[{"key":"/reference_video_urls","kind":"video","role":"reference","multiple":true,"max":1}],
            "referenceLimit":1,"referenceSeconds":15
        }
    }});
    let preflight =
        generation_prepare::prepare(&request["document"], "task", &request["model"], None).unwrap();
    assert!(preflight.get("error").is_none(), "{preflight}");
    assert_eq!(preflight["references"].as_array().unwrap().len(), 2);
    assert_eq!(
        preflight["input"]["reference_video_urls"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    request["uploaded"] = json!([{
        "assetId":"one","sourceAssetIds":["one","two"],"kind":"video","url":"https://cdn.example.com/merged.mp4"
    }]);
    let uploaded = request["uploaded"].take();
    let result = generation_prepare::prepare(
        &request["document"],
        "task",
        &request["model"],
        Some(uploaded),
    )
    .unwrap();
    assert!(result.get("error").is_none(), "{result}");
    assert_eq!(
        result["input"]["reference_video_urls"],
        json!(["https://cdn.example.com/merged.mp4"])
    );
    let prompt = result["input"]["prompt"].as_str().unwrap();
    assert!(
        prompt.contains("Video 1 (1.333–2.667s): 动作参考 · Reveal"),
        "{prompt}"
    );
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
