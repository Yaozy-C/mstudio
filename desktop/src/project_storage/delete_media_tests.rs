use super::tests::Fixture;
use crate::{project_service, projects};
use serde_json::{Value, json};
use std::path::Path;

fn document(id: &str, assets: Value) -> Value {
    json!({"id":id,"name":id,"assets":assets,"nodes":[],"clips":[],"captions":[],"tracks":[],"width":1080,"height":1920,"fps":30,"viewport":{"x":0,"y":0,"scale":1}})
}
#[test]
fn deleting_media_removes_files_and_job_payload_and_recovery_cannot_restore_it() {
    let f = Fixture::new();
    let asset = f.asset("wrong");
    let mut before = document("one", json!([asset]));
    let task = json!({"key":"run","turnId":"turn","jobId":"job","kind":"video","mode":"multi","modelId":"model","prompt":"test","inputs":[],"status":"COMPLETED","resultAssetId":asset.id,"resultAssetIds":[asset.id]});
    before["production"] = json!({"drafts":{"run":task}});
    projects::write_document(&f.store, before.clone(), false).unwrap();
    let job = json!({"id":"job","projectId":"one","status":"COMPLETED","outputCount":1,"asset":asset,"assets":[asset],"result":{"b64_json":"aW1hZ2U="},"outputs":[{"kind":"image","url":"data:image/png;base64,aW1hZ2U="}],"shot":{"canvasGeneration":{"task":task,"x":0,"y":0}}});
    crate::jobs::save(&f.store, &job).unwrap();
    {
        let db = f.store.db.lock().unwrap();
        // A different data-URL prefix still contains the same binary snapshot.
        let image = rig_core::message::UserContent::image_base64(
            "aW1hZ2U=",
            Some(rig_core::message::ImageMediaType::PNG),
            None,
        );
        let mut event = json!({"image":"data:image/jpeg;base64,aW1hZ2U=", "message":{"role":"user","content":[image, {"type":"image","data":{"type":"url","value":format!("file://{}",asset.path)}}]}});
        serde_json::from_value::<rig_core::message::Message>(event["message"].clone()).unwrap();
        db.execute("INSERT INTO agent_events(project_id,turn_id,kind,payload) VALUES('one','turn','tool/result','{}')", []).unwrap();
        let seq = db.last_insert_rowid();
        crate::database::blobs::pack(&db, crate::database::blobs::Owner::Event(seq), &mut event)
            .unwrap();
        db.execute(
            "UPDATE agent_events SET payload=?1 WHERE seq=?2",
            rusqlite::params![event.to_string(), seq],
        )
        .unwrap();
    }
    assert_eq!(f.count("stored_media"), 1);
    let before = project_service::load(&f.store.db.lock().unwrap(), "one").unwrap();
    let mut after = before.clone();
    after["removedAssetIds"] = json!([asset.id]);
    let saved = projects::save_merged(&f.store, after, before).unwrap();
    assert_eq!(saved["assets"], json!([]));
    assert!(!Path::new(&asset.path).exists());
    assert!(!Path::new(&asset.preview).exists());
    assert_eq!(f.count("assets"), 0);
    assert_eq!(f.count("job_content"), 0);
    assert_eq!(f.count("stored_media"), 0);
    assert_eq!(f.count("event_content"), 0);
    assert_eq!(
        std::fs::read_dir(f.store.root.join("reference-media"))
            .unwrap()
            .count(),
        0
    );
    {
        let db = f.store.db.lock().unwrap();
        let raw: String = db
            .query_row(
                "SELECT payload FROM agent_events WHERE project_id='one'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        let event: Value = serde_json::from_str(&raw).unwrap();
        serde_json::from_value::<rig_core::message::Message>(event["message"].clone()).unwrap();
        assert_eq!(event["message"]["content"][0]["type"], "text");
        assert_eq!(event["message"]["content"][1]["type"], "text");
    }
    let stored = crate::jobs::get(&f.store, "job").unwrap();
    assert_eq!(stored["assets"][0], json!({"id":asset.id,"deleted":true}));
    assert!(stored["result"].is_null());
    // Even an already-running worker writing its old snapshot cannot bring
    // the media bytes or project reference back.
    crate::jobs::save(&f.store, &job).unwrap();
    assert_eq!(f.count("job_content"), 0);
    project_service::production::recover(&f.store).unwrap();
    let restored = project_service::load(&f.store.db.lock().unwrap(), "one").unwrap();
    assert_eq!(restored["assets"], json!([]));
    assert_eq!(
        restored["production"]["drafts"]["run"]["resultAssetIds"],
        json!([])
    );
    assert_eq!(
        restored["production"]["drafts"]["run"]["status"],
        "COMPLETED"
    );
}

#[test]
fn deleting_shared_media_only_releases_this_project() {
    let f = Fixture::new();
    let asset = f.asset("shared");
    for id in ["one", "two"] {
        projects::write_document(&f.store, document(id, json!([asset])), false).unwrap();
    }
    let before = project_service::load(&f.store.db.lock().unwrap(), "one").unwrap();
    let mut after = before.clone();
    after["removedAssetIds"] = json!([asset.id]);
    projects::save_merged(&f.store, after, before).unwrap();
    assert!(Path::new(&asset.path).exists());
    assert_eq!(f.count("assets"), 1);
    let db = f.store.db.lock().unwrap();
    assert_eq!(
        project_service::load(&db, "two").unwrap()["assets"][0]["id"],
        asset.id
    );
    let owners: i64 = db
        .query_row(
            "SELECT count(*) FROM project_assets WHERE project_id='one'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(owners, 0);
    drop(db);
    let before = project_service::load(&f.store.db.lock().unwrap(), "two").unwrap();
    let mut after = before.clone();
    after["removedAssetIds"] = json!([asset.id]);
    projects::save_merged(&f.store, after, before).unwrap();
    assert!(!Path::new(&asset.path).exists());
    assert_eq!(f.count("assets"), 0);
}
