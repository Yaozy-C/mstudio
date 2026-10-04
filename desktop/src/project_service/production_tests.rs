use super::tests::fixture;
use super::*;
#[test]
fn asset_tool_contract_rejects_all_errors_before_saving_and_corrected_call_commits_once() {
    let (store, before, _) = fixture();
    let profile = crate::assistant::profiles::builtins()
        .into_iter()
        .find(|p| p.id == "asset-designer")
        .unwrap();
    crate::assistant::history::append_attributed(&store, "p", "Generate", &json!("Generate"), "", "m", Some(&json!({"turnId":"turn","request":{"production":{"projectId":"p","models":{"execution":"confirm"},"instruction":"Generate"}}}))).unwrap();
    let bad = json!({"action":"edit","operations":[{"type":"request_generation","mediaKind":"image","text":"Synthetic reference","mode":"image","modelId":"invalid"}]});
    let result = execute(&store, &profile, "p", "turn", "bad", bad).unwrap();
    assert_eq!(result["code"], "INVALID_ARGS");
    assert_eq!(result["issues"].as_array().unwrap().len(), 6);
    assert_eq!(load(&store.db.lock().unwrap(), "p").unwrap(), before);
    let valid = json!({"action":"edit","operations":[{"op":"request_generation","mediaKind":"image","text":"Synthetic reference","generationPurpose":"asset","references":[]}]});
    let receipt = execute(&store, &profile, "p", "turn", "corrected", valid.clone()).unwrap();
    assert_eq!(receipt["outcome"], "committed", "{receipt}");
    assert_eq!(
        receipt["generationTasks"][0]["status"],
        "AWAITING_CONFIRMATION"
    );
    assert_eq!(
        execute(&store, &profile, "p", "turn", "corrected", valid).unwrap(),
        receipt
    );
    assert_eq!(
        load(&store.db.lock().unwrap(), "p").unwrap()["production"]["drafts"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    std::fs::remove_dir_all(&store.root).unwrap();
}
#[test]
fn outbox_claim_survives_restart_but_reserved_remote_jobs_are_never_resubmitted() {
    let (store, mut doc, _) = fixture();
    doc["production"] = json!({"drafts":{
        "safe":{"key":"safe","status":"READY","modelId":"m","kind":"image","prompt":"Synthetic","inputs":[]},
        "uncertain":{"key":"uncertain","status":"SUBMITTING","submissionId":"remote","jobId":"remote"}
    }});
    crate::projects::write_document(&store, doc, false).unwrap();
    let task = production::claim(&store, "p", "safe").unwrap().unwrap();
    assert!(production::claim(&store, "p", "safe").unwrap().is_none());
    store
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO jobs VALUES('remote','p',?1)",
            [json!({"id":"remote","projectId":"p","status":"UNKNOWN"}).to_string()],
        )
        .unwrap();
    production::recover(&store).unwrap();
    let restored = load(&store.db.lock().unwrap(), "p").unwrap();
    assert_eq!(restored["production"]["drafts"]["safe"]["status"], "READY");
    assert_eq!(
        restored["production"]["drafts"]["safe"]["submissionId"],
        task["submissionId"]
    );
    assert_eq!(
        production::candidates(&store.db.lock().unwrap()).unwrap(),
        vec![("p".into(), "safe".into())]
    );
    // A user cancellation during reference preparation prevents dispatch.
    production::claim(&store, "p", "safe").unwrap();
    let id = task["submissionId"].as_str().unwrap();
    assert!(
        production::patch(
            &store,
            "p",
            "safe",
            id,
            &["UPLOADING"],
            json!({"status":"CANCELLED"})
        )
        .unwrap()
    );
    assert!(
        !production::patch(
            &store,
            "p",
            "safe",
            id,
            &["UPLOADING"],
            json!({"status":"SUBMITTING"})
        )
        .unwrap()
    );
    std::fs::remove_dir_all(&store.root).unwrap();
}
#[test]
fn generation_duration_is_validated_against_the_selected_model_before_saving() {
    let (store, _, profile) = fixture();
    store.set_setting("media-models", &json!([{"id":"video","name":"Video","kind":"video","plugin":"fal","endpoint":"minimax/h3/text-to-video","params":{"resolution":"480P"},"enabled":true}]).to_string()).unwrap();
    crate::assistant::history::append_attributed(&store, "p", "Generate", &json!("Generate"), "", "m", Some(&json!({"turnId":"turn","request":{"production":{"projectId":"p","models":{"video":"video","execution":"automatic"},"instruction":"Generate"}}}))).unwrap();
    let args = json!({"action":"edit","operations":[{"op":"request_generation","mediaKind":"video","text":"Synthetic scene","parameters":{"duration":7,"resolution":"480P"}}]});
    let receipt = execute(&store, &profile, "p", "turn", "generate", args.clone()).unwrap();
    assert_eq!(receipt["outcome"], "committed", "{receipt}");
    assert_eq!(receipt["generationTasks"][0]["parameters"]["duration"], 7);
    let root = store.root.clone();
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    assert_eq!(
        execute(&store, &profile, "p", "turn", "generate", args.clone()).unwrap(),
        receipt
    );
    let mut invalid = args;
    invalid["operations"][0]["parameters"]["duration"] = json!(3);
    let rejected = execute(&store, &profile, "p", "turn", "invalid", invalid).unwrap();
    assert_eq!(rejected["outcome"], "not_executed");
    assert_eq!(
        load(&store.db.lock().unwrap(), "p").unwrap()["production"]["drafts"]
            .as_object()
            .unwrap()
            .len(),
        1
    );
    let model =
        crate::assistant::media_tools::catalog(&store, &json!({"mediaModelId":"video"})).unwrap();
    assert_eq!(
        model["capabilities"]["parameters"]["properties"]["duration"]["minimum"],
        5
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn embedded_generation_accepts_http_references_before_and_after_upload() {
    let (store, mut doc, _) = fixture();
    doc["assets"] = json!([{"id":"image","kind":"image","name":"Reference","path":"/synthetic.png","width":1024,"height":1024,"duration":0}]);
    doc["production"] = json!({"drafts":{"task":{"key":"task","position":{"x":0,"y":0},"kind":"video","mode":"multi","prompt":"Synthetic action","inputs":[{"key":"image","assetId":"image","role":"reference","purpose":"Identity"}],"status":"READY"}}});
    let model = json!({"id":"model","name":"Video","plugin":"fal","endpoint":"minimax/h3/reference-to-video","kind":"video","enabled":true,"params":{}});
    let base = json!({"action":"prepare_generation","document":doc,"taskKey":"task","model":model});
    let preflight = runtime::execute(base.clone()).unwrap();
    assert!(preflight.get("error").is_none(), "{preflight}");
    for url in [
        "https://cdn.example.com/reference.png?token=synthetic",
        "data:image/png;base64,YQ==",
    ] {
        let mut request = base.clone();
        request["uploaded"] = json!([{"assetId":"image","kind":"image","url":url}]);
        let result = runtime::execute(request).unwrap();
        assert!(result.get("error").is_none(), "{result}");
        assert_eq!(result["input"]["reference_image_urls"][0], url);
    }
    for url in [
        "/local/reference.png",
        "file:///local/reference.png",
        "https://user:password@example.com/image.png",
        "not a url",
    ] {
        let mut request = base.clone();
        request["uploaded"] = json!([{"assetId":"image","kind":"image","url":url}]);
        assert!(runtime::execute(request).unwrap().get("error").is_some());
    }
    std::fs::remove_dir_all(&store.root).unwrap();
}
