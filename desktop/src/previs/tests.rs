use super::*;
use crate::{database::Store, projects};
use serde_json::json;

pub(super) fn setup() -> Store {
    let root = std::env::temp_dir().join(format!("mstudio-previs-test-{}", mstudio::media::id()));
    let store = Store::open(root).unwrap();
    projects::write_document(&store,json!({"id":"p","name":"Private previs","assets":[],"nodes":[{"id":"s1","kind":"shot","title":"One","text":"","x":0,"y":0,"shot":{"screenplayId":"script","order":1,"duration":3,"dialogue":""}},{"id":"s2","kind":"shot","title":"Two","text":"","x":0,"y":0,"shot":{"screenplayId":"script","order":2,"duration":3,"dialogue":""}}],"clips":[],"tracks":[],"captions":[],"brief":"","width":720,"height":1280,"fps":24}),true).unwrap();
    store
}
pub(super) fn model(id: &str, name: &str) -> File {
    File {
        id: id.into(),
        kind: "model".into(),
        name: name.into(),
        description: "meters; userData.parts.lid".into(),
        code: "export function createModel({THREE}) { return new THREE.Group(); }".into(),
        revision: 0,
        asset_ids: vec![],
        bindings: Default::default(),
        duration: 3.,
        width: 128,
        height: 128,
        fps: 24,
    }
}
pub(super) fn shot(id: &str, asset: &str) -> File {
    let mut f = model(id, id);
    f.kind = "shot".into();
    f.code="export function createShot({THREE,models}) { return {scene:new THREE.Scene(),camera:new THREE.PerspectiveCamera(),update(t){}}; }".into();
    f.asset_ids = vec![asset.into()];
    f.bindings.insert("Lunch bag".into(), asset.into());
    f
}

#[test]
fn shared_model_edits_replace_the_original_without_modifying_agent_snapshot() {
    let store = setup();
    let before = crate::project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    let db = store.db.lock().unwrap();
    let a = files::save(&db, &store, "p", model("bag", "Lunch bag"), 0).unwrap();
    let s1 = files::save(&db, &store, "p", shot("s1", "bag"), 0).unwrap();
    let s2 = files::save(&db, &store, "p", shot("s2", "bag"), 0).unwrap();
    assert!(files::path(&store, "p", &a).unwrap().is_file());
    let original = files::path(&store, "p", &a).unwrap();
    assert_eq!(original, store.media_root().join("previs/p/bag/model.js"));
    assert_eq!(
        files::path(&store, "p", &s1).unwrap(),
        store.media_root().join("previs/p/s1/scene.js")
    );
    let after = crate::project_service::load(&db, "p").unwrap();
    assert_eq!(before, after);
    let snapshot = crate::assistant::context::project_snapshot(&after, None).to_string();
    assert!(
        !snapshot.contains("createModel")
            && !snapshot.contains("userData.parts")
            && !snapshot.contains("Lunch bag")
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM assets", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM agent_messages", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    let old = runtime::Snapshot::create("p".into(), s1.clone(), vec![a.clone()]).unwrap();
    let mut changed = a.clone();
    changed.code.push_str("\n// revised lid");
    let changed = files::save(&db, &store, "p", changed, 1).unwrap();
    assert_eq!(files::path(&store, "p", &changed).unwrap(), original);
    assert_eq!(std::fs::read_to_string(&original).unwrap(), changed.code);
    assert_eq!(
        std::fs::read_dir(original.parent().unwrap())
            .unwrap()
            .count(),
        1
    );
    assert!(!old.models[0].code.contains("revised lid"));
    assert!(files::save(&db, &store, "p", a, 1).is_err());
    assert_eq!(std::fs::read_to_string(&original).unwrap(), changed.code);
    assert_ne!(
        old.signature,
        runtime::Snapshot::create("p".into(), s1, vec![changed.clone()])
            .unwrap()
            .signature
    );
    assert_ne!(
        old.signature,
        runtime::Snapshot::create("p".into(), s2, vec![changed])
            .unwrap()
            .signature
    );
    drop(db);
    crate::project_storage::remove(&store, "p").unwrap();
    assert!(!store.media_root().join("previs/p").exists());
    let _ = std::fs::remove_dir_all(&store.root);
}
#[test]
fn model_names_and_dependency_scope_are_validated() {
    let store = setup();
    let db = store.db.lock().unwrap();
    files::save(&db, &store, "p", model("bag", "Lunch bag"), 0).unwrap();
    assert!(files::save(&db, &store, "p", model("other", "lunch BAG"), 0).is_err());
    assert!(files::save(&db, &store, "p", shot("s1", "missing"), 0).is_err());
    assert!(files::path(&store, "../p", &model("bag", "Bag")).is_err());
    assert!(files::save(&db, &store, "p", shot("unknown-shot", "bag"), 0).is_err());
    drop(db);
    let _ = std::fs::remove_dir_all(&store.root);
}
#[test]
fn runtime_serves_only_frozen_declared_modules() {
    let server = runtime::PrevisRuntime::start().unwrap();
    let a = model("bag", "Lunch bag");
    let s = shot("s1", "bag");
    let frozen = runtime::Snapshot::create("p".into(), s, vec![a]).unwrap();
    let id = server.open(frozen.clone()).unwrap();
    assert!(server.get(&id, "other").is_err());
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let url = format!("{}/{id}/manifest.json", server.address);
        let response = reqwest::get(url).await.unwrap();
        assert!(
            !response
                .headers()
                .contains_key("access-control-allow-origin")
        );
        assert!(
            response.headers()["content-security-policy"]
                .to_str()
                .unwrap()
                .contains("default-src 'none'")
        );
        for (file, marker) in [
            ("runtime.js", "./three.module.js"),
            ("three.module.js", "./three.core.min.js"),
            ("three.core.min.js", "SPDX-License-Identifier: MIT"),
            ("three.LICENSE.txt", "Permission is hereby granted"),
        ] {
            let response = reqwest::get(format!("{}/{id}/{file}", server.address))
                .await
                .unwrap();
            assert_eq!(response.status(), 200, "missing runtime module: {file}");
            assert!(response.text().await.unwrap().contains(marker));
        }
        let data: serde_json::Value = response.json().await.unwrap();
        assert_eq!(data["signature"], frozen.signature);
        for path in ["../p/scene.js", "unknown.js", "model-unknown.js"] {
            let response = reqwest::get(format!("{}/{id}/{path}", server.address))
                .await
                .unwrap();
            assert_eq!(response.status(), 404);
        }
        assert_eq!(
            reqwest::get(format!("{}/unknown/shot.js", server.address))
                .await
                .unwrap()
                .status(),
            404
        );
    });
}
#[test]
fn only_a_manually_shared_video_copy_enters_project_and_agent_media() {
    let store = setup();
    let out = store.media_root().join("exports/private-previs");
    std::fs::create_dir_all(&out).unwrap();
    crate::project_storage::track_file(&store, "p", &out).unwrap();
    let source = out.join("preview.mp4");
    mstudio::media::run(
        mstudio::media::command("ffmpeg")
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=c=blue:s=64x64:r=24:d=0.5",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(&source),
    )
    .unwrap();
    let mut asset = mstudio::media::import(&source, &out.join("media")).unwrap();
    asset.name = "Shot 04 | 7.3–10.0s 急刹验证 · 预演".into();
    let private = asset.path.clone();
    let export = render::Export {
        id: "export-one".into(),
        file_id: "s1".into(),
        file_revision: 1,
        model_revisions: Default::default(),
        signature: "frozen".into(),
        asset,
        shared_asset_id: None,
    };
    store
        .db
        .lock()
        .unwrap()
        .execute(
            "INSERT INTO previs_exports VALUES('p','export-one',?1)",
            [serde_json::to_string(&export).unwrap()],
        )
        .unwrap();
    assert!(store.assets().unwrap().is_empty());
    let reply = sharing::set_shared(&store, "p", "export-one", true).unwrap();
    let public = reply["assetId"].as_str().unwrap().to_owned();
    assert_eq!(reply["asset"]["id"], public);
    assert_eq!(reply["asset"]["inLibrary"], true);
    assert_eq!(reply["asset"]["name"], format!("{}.mp4", export.asset.name));
    assert!(reply["asset"]["duration"].as_f64().unwrap() > 0.);
    let reused = sharing::set_shared(&store, "p", "export-one", true).unwrap();
    assert_eq!(reused["asset"], reply["asset"]);
    assert_ne!(public, export.asset.id);
    let doc = crate::project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    assert_eq!(doc["assets"].as_array().unwrap().len(), 1);
    assert_eq!(doc["nodes"].as_array().unwrap().len(), 2);
    assert_ne!(doc["assets"][0]["path"], private);
    assert!(std::path::Path::new(&private).exists());
    sharing::set_shared(&store, "p", "export-one", false).unwrap();
    let doc = crate::project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    assert!(doc["assets"].as_array().unwrap().is_empty());
    let reply = sharing::set_shared(&store, "p", "export-one", true).unwrap();
    let public = reply["assetId"].as_str().unwrap().to_owned();
    {
        let db = store.db.lock().unwrap();
        let before = crate::project_service::load(&db, "p").unwrap();
        let mut after = before.clone();
        after["clips"] = json!([{"id":"clip","assetId":public}]);
        crate::project_service::persist(&db, &before, &after).unwrap();
    }
    assert!(sharing::set_shared(&store, "p", "export-one", false).is_err());
    {
        let db = store.db.lock().unwrap();
        let before = crate::project_service::load(&db, "p").unwrap();
        let mut after = before.clone();
        after["clips"] = json!([]);
        after["assets"] = json!([]);
        after["removedAssetIds"] = json!([public]);
        crate::project_service::persist(&db, &before, &after).unwrap();
    }
    crate::project_storage::resume_cleanup(&store.db.lock().unwrap(), &store.media_root()).unwrap();
    assert!(
        std::path::Path::new(&private).exists(),
        "ordinary asset deletion must preserve private render"
    );
    crate::project_storage::remove(&store, "p").unwrap();
    assert!(!out.exists());
    let _ = std::fs::remove_dir_all(store.root);
}

#[test]
fn deleted_shots_do_not_permanently_lock_shared_models() {
    let store = setup();
    let db = store.db.lock().unwrap();
    files::save(&db, &store, "p", model("bag", "Lunch bag"), 0).unwrap();
    files::save(&db, &store, "p", shot("s1", "bag"), 0).unwrap();
    assert!(files::model_in_use(&db, "p", "bag").unwrap());
    let before = crate::project_service::load(&db, "p").unwrap();
    let mut after = before.clone();
    after["nodes"]
        .as_array_mut()
        .unwrap()
        .retain(|n| n["id"] != "s1");
    crate::project_service::persist(&db, &before, &after).unwrap();
    assert!(!files::model_in_use(&db, "p", "bag").unwrap());
    drop(db);
    let _ = std::fs::remove_dir_all(store.root);
}
