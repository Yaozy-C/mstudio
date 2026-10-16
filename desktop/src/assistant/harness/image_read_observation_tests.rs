use super::*;
use crate::{assistant::profiles, project_service};

fn fixture() -> (Store, Profile, profiles::AgentProfile) {
    let store = Store::open(std::env::temp_dir().join(format!(
        "mstudio-image-observation-{}",
        mstudio::media::id()
    )))
    .unwrap();
    let path = store.media_root().join("assets/result.png");
    std::fs::create_dir_all(path.parent().unwrap()).unwrap();
    std::fs::write(&path, [137, 80, 78, 71]).unwrap();
    let asset = json!({"id":"result","name":"result.png","kind":"image","path":path,"preview":"","duration":0,"width":1,"height":1,"hasAudio":false,"generated":true});
    store
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO assets VALUES('result',?1)",
            [asset.to_string()],
        )
        .unwrap();
    let doc = json!({"id":"p","name":"Test","revision":0,"brief":"","width":1280,"height":720,"fps":30,
        "assets":[asset],"nodes":[],"clips":[],"tracks":[],"captions":[],
        "production":{"drafts":{"task":{"key":"task","kind":"image","mode":"single","inputs":[],"prompt":"Original image","modelId":"","status":"COMPLETED","resultAssetIds":["result"],"turnId":"original","createdAt":1}}}});
    crate::projects::write_document(&store, doc, true).unwrap();
    let profile = Profile {
        inputs: crate::assistant::config::Inputs {
            image: true,
            ..Default::default()
        },
        ..Default::default()
    };
    let agent = profiles::builtins()
        .into_iter()
        .find(|p| p.id == "image")
        .unwrap();
    crate::assistant::history::append_attributed(&store, "p", "Regenerate", &json!("Regenerate"), "", "m",
        Some(&json!({"turnId":"image-child","request":{"production":{"projectId":"p","models":{"execution":"confirm"},"instruction":"Regenerate"}}}))).unwrap();
    let task = project_service::execute(
        &store,
        &agent,
        "p",
        "image-child",
        "read-task",
        json!({"action":"inspect","section":"generation","taskKey":"task"}),
    )
    .unwrap();
    assert_eq!(task["items"][0]["id"], "task", "{task}");
    (store, profile, agent)
}

fn regenerate(store: &Store, agent: &profiles::AgentProfile) -> Value {
    let args = super::super::operation_tools::decode(
        agent,
        "mstudio_regenerate_generation",
        json!({"taskKey":"task","keepPreviousResults":false}),
    )
    .unwrap();
    project_service::execute(store, agent, "p", "image-child", "regenerate", args).unwrap()
}

#[test]
fn image_agent_can_replace_a_visually_read_result_without_a_second_metadata_read() {
    let (store, profile, agent) = fixture();
    let conflict = regenerate(&store, &agent);
    assert_eq!(conflict["conflicts"][0]["code"], "TARGET_NOT_OBSERVED");
    assert_eq!(conflict["conflicts"][0]["target"], "asset:result");
    assert_eq!(
        read(
            &store,
            "p",
            "image-child",
            &profile,
            &json!({"assetId":"result"})
        )["ok"],
        true
    );
    let doc = project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    let keys = std::collections::BTreeSet::from(["asset:result".to_owned()]);
    let other_turn = project_service::observations::check(
        &store.db.lock().unwrap(),
        "p",
        "another-child",
        &keys,
        &doc,
    )
    .unwrap()
    .unwrap();
    assert_eq!(other_turn["conflicts"][0]["code"], "TARGET_NOT_OBSERVED");
    let receipt = regenerate(&store, &agent);
    assert_eq!(receipt["applied"], true, "{receipt}");
    let saved = project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    assert_eq!(saved["assets"][0]["discarded"], true);
    assert_eq!(
        saved["production"]["drafts"]["task"]["status"],
        "AWAITING_CONFIRMATION"
    );
    assert_eq!(regenerate(&store, &agent), receipt);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn image_agent_cannot_replace_a_result_changed_after_its_visual_read() {
    let (store, profile, agent) = fixture();
    assert_eq!(
        read(
            &store,
            "p",
            "image-child",
            &profile,
            &json!({"assetId":"result"})
        )["ok"],
        true
    );
    let before = project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    let mut changed = before.clone();
    changed["assets"][0]["name"] = json!("Human renamed result");
    crate::projects::save_merged(&store, changed, before).unwrap();
    let saved = project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    let conflict = regenerate(&store, &agent);
    assert_eq!(
        conflict["conflicts"][0]["code"], "TARGET_CHANGED",
        "{conflict}"
    );
    assert_eq!(conflict["outcome"], "not_executed");
    assert_eq!(
        project_service::load(&store.db.lock().unwrap(), "p").unwrap(),
        saved
    );
    assert_eq!(
        read(
            &store,
            "p",
            "image-child",
            &profile,
            &json!({"assetId":"result"})
        )["ok"],
        true
    );
    assert_eq!(regenerate(&store, &agent)["applied"], true);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
