//! Coordinator-level structure and task-edit coverage, split out to keep
//! each maintained test module within the source limit.
use super::tests::{decode, profile};
use serde_json::json;

#[test]
fn screenplay_creation_rejects_body_text_and_commits_structured_content() {
    let p = profile("writer");
    let input = json!({"id":"script","title":"Lunch",
        "script":[{"id":"p1","title":"Opening","action":"Open bag","dialogue":"","sound":"","duration":3}]});
    for (name, key, value) in [
        ("mstudio_add_screenplay", "paragraphOrder", json!(["p1"])),
        ("mstudio_add_screenplay", "removeParagraphIds", json!([])),
        ("mstudio_add_screenplay", "scriptMode", json!("replace")),
        ("mstudio_update_screenplay", "scriptMode", json!("replace")),
    ] {
        let mut invalid = input.clone();
        invalid[key] = value;
        assert!(decode(&p, name, invalid).unwrap().is_err());
    }
    let mut invalid = input.clone();
    invalid["text"] = json!("Extra explanation");
    let error = decode(&p, "mstudio_add_screenplay", invalid)
        .unwrap()
        .unwrap_err();
    assert_eq!(error["outcome"], "not_executed");
    assert!(
        error["issues"]
            .as_array()
            .unwrap()
            .iter()
            .any(|issue| issue["path"] == "$.text")
    );

    let root = std::env::temp_dir().join(format!("mstudio-script-{}", mstudio::media::id()));
    let store = crate::database::Store::open(root.clone()).unwrap();
    crate::projects::write_document(
        &store,
        json!({
            "id":"p","name":"Synthetic","revision":0,"brief":"",
            "width":1280,"height":720,"fps":30,
            "assets":[],"nodes":[],"clips":[],"tracks":[],"captions":[]
        }),
        true,
    )
    .unwrap();
    let args = decode(&p, "mstudio_add_screenplay", input)
        .unwrap()
        .unwrap();
    let receipt =
        crate::project_service::execute(&store, &p, "p", "turn", "turn:create", args).unwrap();
    assert_eq!(receipt["outcome"], "committed", "{receipt}");
    let saved = crate::project_service::execute(
        &store,
        &p,
        "p",
        "turn",
        "read",
        json!({"action":"inspect","nodeIds":["script"],"fields":["screenplay"]}),
    )
    .unwrap()
    .to_string();
    assert!(saved.contains("Open bag"), "{saved}");
    let duplicate = json!({"id":"second-script","title":"Wrong second screenplay",
        "script":[{"id":"reference","title":"Elements","action":"Notes","dialogue":"","sound":"","duration":0.01}]});
    let args = decode(&p, "mstudio_add_screenplay", duplicate.clone())
        .unwrap()
        .unwrap();
    let rejected =
        crate::project_service::execute(&store, &p, "p", "turn", "turn:duplicate", args).unwrap();
    assert_eq!(rejected["outcome"], "not_executed");
    assert!(
        rejected["error"]
            .as_str()
            .unwrap()
            .contains("mstudio_update_screenplay")
    );
    let update = json!({"id":"script","title":"Rewritten script",
        "script":[{"id":"p1","title":"Opening","action":"Close bag","dialogue":"","sound":"Zip","duration":3}]});
    let args = decode(&p, "mstudio_update_screenplay", update)
        .unwrap()
        .unwrap();
    let replaced =
        crate::project_service::execute(&store, &p, "p", "turn", "turn:replace", args).unwrap();
    assert_eq!(replaced["outcome"], "committed", "{replaced}");
    let document = crate::project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    assert_eq!(document["nodes"].as_array().unwrap().len(), 1);
    assert_eq!(document["nodes"][0]["id"], "script");
    assert_eq!(document["nodes"][0]["title"], "Rewritten script");
    assert_eq!(document["nodes"][0]["screenplay"]["script"][0]["id"], "p1");
    assert_eq!(
        document["nodes"][0]["screenplay"]["script"][0]["action"],
        "Close bag"
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn director_add_shot_requires_structure_and_coordinator_cannot_author_media() {
    let director = profile("director");
    let failed = decode(
        &director,
        "mstudio_add_shot",
        json!({
            "id":"shot", "title":"Design", "text":"Short proposal", "duration":15
        }),
    )
    .unwrap()
    .unwrap_err();
    let paths: Vec<_> = failed["issues"]
        .as_array()
        .unwrap()
        .iter()
        .map(|issue| issue["path"].as_str().unwrap())
        .collect();
    assert!(paths.contains(&"$.screenplayId"));
    assert!(paths.contains(&"$.order"));
    assert_eq!(failed["outcome"], "not_executed");
    let coordinator = profile("coordinator");
    for name in [
        "mstudio_add_shot",
        "mstudio_add_screenplay",
        "mstudio_update_generation",
        "mstudio_generate_video",
    ] {
        assert!(decode(&coordinator, name, json!({})).is_none(), "{name}");
    }
}

#[test]
fn regeneration_accepts_replacement_prompt_and_references_without_creating_a_task() {
    let p = profile("production");
    let value = decode(
        &p,
        "mstudio_regenerate_generation",
        json!({
            "taskKey":"existing", "prompt":"Corrected", "references":[
                {"assetId":"approved", "role":"first-frame", "purpose":"Product identity"}
            ]
        }),
    )
    .unwrap()
    .unwrap();
    assert_eq!(value["operations"][0]["op"], "regenerate_generation");
    assert_eq!(value["operations"][0]["taskKey"], "existing");
    assert_eq!(value["operations"][0]["text"], "Corrected");
    assert_eq!(
        value["operations"][0]["references"][0]["assetId"],
        "approved"
    );
    assert!(value["operations"][0].get("prompt").is_none());
    assert!(
        decode(
            &p,
            "mstudio_regenerate_generation",
            json!({"taskKey":"existing"})
        )
        .unwrap()
        .is_ok()
    );
}

#[test]
fn coordinator_has_task_archival_but_no_generation_or_prompt_authoring() {
    let p = profile("coordinator");
    let args = decode(
        &p,
        "mstudio_archive_generation",
        json!({"taskKey":"old","archived":true}),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        args["operations"][0],
        json!({"op":"archive_generation","taskKey":"old","archived":true})
    );
    assert!(
        decode(
            &p,
            "mstudio_archive_generation",
            json!({"taskKey":"old","archived":"true"})
        )
        .unwrap()
        .is_err()
    );
    assert!(decode(&p, "mstudio_update_generation", json!({})).is_none());
    assert!(decode(&p, "mstudio_generate_video", json!({})).is_none());
}
