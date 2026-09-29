use super::*;
use crate::models::connections::{self, ServiceConnection};
#[test]
fn queued_job_keeps_its_connection_when_model_switches() {
    let root = std::env::temp_dir().join(format!("mstudio-job-service-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    {
        let db = store.db.lock().unwrap();
        db.execute(
            "INSERT INTO settings VALUES('fal-key','original-secret')",
            [],
        )
        .unwrap();
        let model = serde_json::json!([{"id":"image","name":"Image","kind":"image","plugin":"fal","endpoint":"fal-ai/test","params":{},"enabled":true}]);
        db.execute(
            "INSERT INTO settings VALUES('media-models',?1)",
            [model.to_string()],
        )
        .unwrap();
        db.execute(
            "INSERT INTO projects VALUES('project','project','{}',0)",
            [],
        )
        .unwrap();
        let job = serde_json::json!({"id":"job","mediaModelId":"image","providerId":"fal","endpoint":"fal-ai/test","status":"IN_QUEUE"});
        db.execute(
            "INSERT INTO jobs VALUES('job','project',?1)",
            [job.to_string()],
        )
        .unwrap();
    }
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    let job: Value;
    {
        let db = store.db.lock().unwrap();
        job = serde_json::from_str(
            &db.query_row("SELECT data FROM jobs WHERE id='job'", [], |r| {
                r.get::<_, String>(0)
            })
            .unwrap(),
        )
        .unwrap();
        assert!(job["connectionId"].is_string());
        connections::save(
            &db,
            ServiceConnection {
                id: "second".into(),
                name: "Second account".into(),
                kind: "fal".into(),
                endpoint: "https://queue.fal.run".into(),
                has_key: false,
                model_count: 0,
            },
            Some("second-secret".into()),
            false,
        )
        .unwrap();
        let mut models = crate::models::media::read(&db).unwrap();
        models[0].connection_id = Some("second".into());
        db.execute(
            "UPDATE settings SET value=?1 WHERE key='media-models'",
            [serde_json::to_string(&models).unwrap()],
        )
        .unwrap();
    }
    assert_eq!(credential(&store, &job).unwrap(), "original-secret");
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    assert_eq!(credential(&store, &job).unwrap(), "original-secret");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
