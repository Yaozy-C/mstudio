use super::*;
use crate::project_service::{production, tests::fixture};

fn request(plugin: &str) -> (Value, Value) {
    let document = json!({"id":"p","name":"Fixture","width":1280,"height":720,"revision":3,"nodes":[],"assets":[{"id":"reference","kind":"image","usage":"asset","duration":0,"width":1024,"height":1024}],"production":{"drafts":{"run":{
        "key":"run","title":"  Portrait  ","kind":"image","mode":"multi","prompt":"  保留产品外观\n柔和光线  ","position":{"x":12,"y":34},"submissionId":"submission",
        "inputs":[{"key":"ref","assetId":"reference","role":"reference","purpose":"产品外观"}]
    }}}});
    let (endpoint, params) = match plugin {
        "codex-image" => (
            "codex://local/images",
            json!({"model":"gpt-image-2","size":"auto","quality":"auto","image":["data:image/png;base64,aGlkZGVu"]}),
        ),
        "gemini-native" => (
            "https://generativelanguage.googleapis.com",
            json!({"model":"gemini-image","image_urls":["data:image/png;base64,aGlkZGVu"]}),
        ),
        _ => (
            "vendor/image",
            json!({"seed":123,"input":{"images":["https://hidden.example/ref.png"]}}),
        ),
    };
    let mut model = json!({"id":"model","name":"Image","kind":"image","enabled":true,"plugin":plugin,"endpoint":endpoint,"params":params});
    if plugin == "fal" {
        model["capabilities"] = json!({"references":[{"key":"/input/images","kind":"image","role":"reference","multiple":true,"max":8}],"controls":{"imageSize":{"path":"/input/size","min":1,"max":4096},"resolution":{"path":"/input/resolution","values":["2K","4K"]}}});
    }
    (document, model)
}

#[test]
fn native_generation_matches_existing_request_contract() {
    for plugin in ["codex-image", "gemini-native", "fal"] {
        let (mut doc, model) = request(plugin);
        doc["production"]["drafts"]["run"]["parameters"] = if plugin == "gemini-native" {
            json!({"aspectRatio":"9:16","resolution":"4K"})
        } else {
            json!({"width":1024,"height":1536,"resolution":"4K"})
        };
        for uploaded in [
            None,
            Some(
                json!([{"assetId":"reference","kind":"image","url":"data:image/png;base64,YQ=="}]),
            ),
        ] {
            let native = prepare(&doc, "run", &model, uploaded.clone()).unwrap();
            let url = "data:image/png;base64,YQ==";
            assert_eq!(
                native["input"]["prompt"],
                "保留产品外观\n柔和光线\n\nImage 1: 内容参考 · 产品外观"
            );
            assert_eq!(
                native["shot"]["canvasGeneration"],
                json!({"task":doc["production"]["drafts"]["run"],"x":12,"y":34})
            );
            assert_eq!(native["shot"]["submissionId"], "submission");
            assert_eq!(native["shot"]["projectRevision"], 3);
            match plugin {
                "codex-image" => {
                    assert_eq!(native["input"]["image"], json!([url]));
                    assert_eq!(native["input"]["size"], "2048x1152");
                    assert_eq!(native["input"]["quality"], "high");
                }
                "gemini-native" => {
                    assert_eq!(native["input"]["image_urls"], json!([url]));
                    assert_eq!(
                        native["input"]["generationConfig"]["imageConfig"],
                        json!({"aspectRatio":"9:16","imageSize":"4K"})
                    );
                }
                _ => {
                    assert_eq!(
                        native["input"]["input"]["images"],
                        json!([if uploaded.is_some() {
                            url
                        } else {
                            "https://example.invalid/validate"
                        }])
                    );
                    assert_eq!(
                        native["input"]["input"]["size"],
                        json!({"width":1024,"height":1536})
                    );
                    assert_eq!(native["input"]["input"]["resolution"], "4K");
                }
            }
        }
    }
}

#[test]
fn six_full_size_reference_images_are_prepared_natively_without_vm_roundtrip() {
    let (store, mut doc, _) = fixture();
    let (_, model) = request("codex-image");
    let references: Vec<_> = (0..6).map(|i| json!({"key":format!("ref{i}"),"assetId":format!("image{i}"),"role":"reference","purpose":"保留产品外观"})).collect();
    doc["assets"] = (0..6).map(|i| json!({"id":format!("image{i}"),"kind":"image","usage":"asset","width":1254,"height":1254,"duration":0})).collect();
    doc["production"] = json!({"drafts":{"run":{"key":"run","kind":"image","mode":"multi","prompt":"保留产品外观，柔和光线","position":{"x":12,"y":34},"inputs":references}}});
    crate::projects::write_document(&store, doc.clone(), false).unwrap();
    let uploaded: Value = (0..6).map(|i| json!({"assetId":format!("image{i}"),"kind":"image","url":format!("data:image/png;base64,{}", "YWJj".repeat(800_000))})).collect();
    let native = production::prepare(&store, "p", "run", &model, Some(uploaded.clone())).unwrap();
    assert_eq!(native["input"]["image"].as_array().unwrap().len(), 6);
    for i in 0..6 {
        assert_eq!(native["input"]["image"][i], uploaded[i]["url"]);
    }
    assert_eq!(native["shot"]["canvasGeneration"]["x"], 12);
    assert_eq!(native["shot"]["canvasGeneration"]["y"], 34);
    assert_eq!(native["shot"]["title"], "未命名");
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn native_generation_rejects_missing_archived_and_unsupported_references() {
    let (doc, mut model) = request("codex-image");
    for change in [
        "missing",
        "annotationSourceId",
        "inLibrary",
        "frame",
        "hiddenFromList",
        "ownerId",
        "role",
    ] {
        let mut doc = doc.clone();
        match change {
            "missing" => doc["assets"][0]["missing"] = json!(true),
            "annotationSourceId" => doc["assets"][0]["annotationSourceId"] = json!("original"),
            "inLibrary" => doc["assets"][0]["inLibrary"] = json!(false),
            "frame" => doc["production"]["drafts"]["run"]["inputs"][0]["frame"] = json!(true),
            "hiddenFromList" => doc["production"]["drafts"]["run"]["hiddenFromList"] = json!(true),
            "ownerId" => doc["production"]["drafts"]["run"]["ownerId"] = json!("removed"),
            "role" => {
                doc["production"]["drafts"]["run"]["inputs"][0]["role"] = json!("first-frame")
            }
            _ => unreachable!(),
        }
        assert!(prepare(&doc, "run", &model, None).is_err(), "{change}");
    }
    for url in [
        "file:///tmp/image.png",
        "https://example.com/ref.png",
        "data:text/html;base64,YQ==",
        "data:image/gif;base64,YQ==",
    ] {
        assert!(
            prepare(
                &doc,
                "run",
                &model,
                Some(json!([{"assetId":"reference","kind":"image","url":url}]))
            )
            .is_err()
        );
    }
    assert!(prepare(&doc, "run", &model, Some(json!([]))).is_err());
    model["capabilities"] = json!({"references":[]});
    assert!(prepare(&doc, "run", &model, None).is_err());
}

#[test]
fn native_generation_handles_merged_videos_and_nested_parameters() {
    let (mut doc, _) = request("fal");
    doc["assets"] = json!([{"id":"one","kind":"video","usage":"asset","duration":1.333333},{"id":"two","kind":"video","usage":"asset","duration":1.333333}]);
    doc["production"]["drafts"]["run"]["kind"] = json!("video");
    doc["production"]["drafts"]["run"]["inputs"] = json!([{"key":"one","assetId":"one","role":"video-reference","purpose":"Opening"},{"key":"two","assetId":"two","role":"video-reference","purpose":"Reveal"}]);
    doc["production"]["drafts"]["run"]["parameters"] = json!({"duration":7});
    let model = json!({"id":"video","name":"Video","kind":"video","plugin":"fal","endpoint":"vendor/video","enabled":true,"params":{},"capabilities":{"references":[{"key":"/input/videos","kind":"video","role":"reference","multiple":true,"max":1}],"referenceLimit":1,"referenceSeconds":15,"controls":{"duration":{"path":"/input/duration","min":5,"max":15}}}});
    let uploaded = json!([{"assetId":"one","sourceAssetIds":["one","two"],"kind":"video","url":"https://cdn.example/merged.mp4"}]);
    let native = prepare(&doc, "run", &model, Some(uploaded.clone())).unwrap();
    assert_eq!(native["references"].as_array().unwrap().len(), 2);
    assert_eq!(
        native["input"]["input"]["videos"],
        json!(["https://cdn.example/merged.mp4"])
    );
    assert!(
        native["input"]["prompt"]
            .as_str()
            .unwrap()
            .contains("Video 1 (1.333–2.667s): 动作参考 · Reveal")
    );
    assert_eq!(native["input"]["input"]["duration"], 7);
    doc["production"]["drafts"]["run"]["parameters"]["duration"] = json!(3);
    assert!(prepare(&doc, "run", &model, None).is_err());
}

#[test]
fn native_generation_supports_text_only_templates_without_hidden_references() {
    let (mut doc, mut model) = request("fal");
    doc["production"]["drafts"]["run"]["inputs"] = json!([]);
    let native = prepare(&doc, "run", &model, Some(json!([]))).unwrap();
    assert!(native["input"].pointer("/input/images").is_none());
    assert_eq!(native["input"]["prompt"], "保留产品外观\n柔和光线");
    model["plugin"] = json!("http-json");
    model["endpoint"] = json!("https://example.com/generate");
    model["http"] = json!({"outputPointer":"/image"});
    model.as_object_mut().unwrap().remove("capabilities");
    model["params"] = json!({"payload":[{"text":"prefix {{prompt}}"}],"count":2,"enabled":true});
    let native = prepare(&doc, "run", &model, Some(json!([]))).unwrap();
    assert_eq!(
        native["input"],
        json!({"payload":[{"text":"prefix 保留产品外观\n柔和光线"}],"count":2,"enabled":true})
    );
    assert!(native["input"].get("prompt").is_none());
}

#[test]
fn native_generation_enforces_declared_limits_and_revalidates_uploaded_files() {
    let (mut doc, mut model) = request("fal");
    let second = doc["production"]["drafts"]["run"]["inputs"][0].clone();
    doc["production"]["drafts"]["run"]["inputs"]
        .as_array_mut()
        .unwrap()
        .push(second);
    model["capabilities"]["references"][0]["max"] = json!(1);
    assert!(prepare(&doc, "run", &model, None).is_err());
    model["capabilities"]["references"][0]["max"] = json!(8);
    model["capabilities"]["referenceLimit"] = json!(1);
    assert!(prepare(&doc, "run", &model, None).is_err());
    let (doc, model) = request("fal");
    for file in [
        json!({"assetId":"reference","kind":"image"}),
        json!({"assetId":"reference","kind":"video","url":"https://example.com/image"}),
    ] {
        assert!(prepare(&doc, "run", &model, Some(json!([file]))).is_err());
    }
}

#[test]
fn native_generation_resolves_nested_array_and_escaped_parameter_paths() {
    let (mut doc, mut model) = request("fal");
    model["params"] =
        json!({"input":{"images":["https://hidden.example/image.png"]},"settings":[]});
    model["capabilities"]["controls"]["resolution"]["path"] = json!("/settings/0/size~1label");
    doc["production"]["drafts"]["run"]["parameters"] = json!({"resolution":"4K"});
    let native = prepare(&doc, "run", &model, None).unwrap();
    assert_eq!(native["input"]["settings"], json!([{"size/label":"4K"}]));
    doc["production"]["drafts"]["run"]["parameters"] = json!({"width":1024});
    assert!(prepare(&doc, "run", &model, None).is_err());
}

#[test]
fn per_task_options_reach_wan_codex_and_fal_requests_without_editing_connections() {
    for (plugin, endpoint, params, kind, options, checks) in [
        (
            "dashscope",
            "https://space.cn-beijing.maas.aliyuncs.com/api/v1/services/aigc/video-generation/video-synthesis",
            json!({"model":"wan3.0-video","parameters":{"audio":true}}),
            "video",
            json!({"audio":false}),
            vec![("/parameters/audio", json!(false))],
        ),
        (
            "codex-image",
            "codex://local/images",
            json!({"model":"gpt-image-2","quality":"high"}),
            "image",
            json!({"quality":"xhigh","n":2}),
            vec![("/quality", json!("xhigh")), ("/n", json!(2))],
        ),
        (
            "fal",
            "openai/gpt-image-2.5/flare/text-to-image",
            json!({"num_images":1,"output_format":"png"}),
            "image",
            json!({"num_images":2,"output_format":"jpeg"}),
            vec![("/num_images", json!(2)), ("/output_format", json!("jpeg"))],
        ),
    ] {
        let document = json!({"width":1280,"height":720,"assets":[],"nodes":[],"production":{"drafts":{"run":{"key":"run","kind":kind,"mode":"single","prompt":"Product on a warm background","inputs":[],"parameters":{"options":options}}}}});
        let model = json!({"id":"fixture","name":"Fixture","kind":kind,"plugin":plugin,"endpoint":endpoint,"params":params,"enabled":true});
        let prepared = prepare(&document, "run", &model, None).unwrap();
        for (path, expected) in checks {
            assert_eq!(
                prepared["input"].pointer(path),
                Some(&expected),
                "{plugin} {path}"
            );
        }
        assert_eq!(model["params"], params);
    }
}

#[test]
fn research_is_valid_for_exploration_but_cannot_leak_into_a_shot_task() {
    let (mut doc, model) = request("fal");
    doc["assets"][0]["usage"] = json!("material");
    assert!(prepare(&doc, "run", &model, None).is_ok());
    doc["nodes"] = json!([{"id":"shot","kind":"shot","shot":{}}]);
    doc["production"]["drafts"]["run"]["targetNodeId"] = json!("shot");
    doc["production"]["drafts"]["run"]["ownerId"] = json!("shot");
    assert!(
        prepare(&doc, "run", &model, None)
            .unwrap_err()
            .to_string()
            .contains("此资源尚未确认为镜头资产")
    );
    doc["assets"][0]["usage"] = json!("asset");
    assert!(prepare(&doc, "run", &model, None).is_ok());
}
