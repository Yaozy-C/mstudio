//! Coordinator-level structure and task-edit coverage, split out to keep
//! each maintained test module within the source limit.
use super::tests::{decode, profile};
use serde_json::json;

#[test]
fn screenplay_creation_rejects_body_text_and_commits_structured_content() {
    let p = profile("writer");
    let input = json!({"id":"script","title":"Lunch",
        "script":[{"id":"p1","title":"Opening","action":"Open bag","duration":3}]});
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
            ], "keepPreviousResults":false
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
