use super::*;
use crate::models::connections::{self, ServiceConnection};

fn connection(db: &rusqlite::Connection, id: &str, key: &str) {
    connections::save(
        db,
        ServiceConnection {
            id: id.into(),
            name: id.into(),
            kind: "fal".into(),
            endpoint: "https://queue.fal.run".into(),
            has_key: false,
            model_count: 0,
        },
        Some(key.into()),
        false,
    )
    .unwrap();
}

#[test]
fn queued_job_keeps_its_explicit_connection_when_the_model_changes() {
    let (store, _, _) = crate::project_service::tests::fixture();
    let root = store.root.clone();
    {
        let db = store.conn().unwrap();
        connection(&db, "original", "original-secret");
        connection(&db, "second", "second-secret");
        let model = json!([{"id":"image","name":"Image","kind":"image","plugin":"fal","connectionId":"original","endpoint":"fal-ai/test","params":{},"enabled":true}]);
        db.execute(
            "INSERT INTO settings VALUES('media-models',?1)",
            [model.to_string()],
        )
        .unwrap();
    }
    let job = json!({"id":"job","projectId":"p","providerId":"fal","mediaModelId":"image",
        "connectionId":"original","connectionEndpoint":"https://queue.fal.run",
        "endpoint":"fal-ai/test","input":{},"status":"IN_QUEUE"});
    assert!(reserve(&store, &job).unwrap());
    {
        let db = store.conn().unwrap();
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
    assert_eq!(
        credential(&store, &get(&store, "job").unwrap()).unwrap(),
        "original-secret"
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn a_missing_job_connection_is_a_validation_error() {
    let (store, _, _) = crate::project_service::tests::fixture();
    let root = store.root.clone();
    connection(
        &store.conn().unwrap(),
        "only-account",
        "never-guess-this-key",
    );
    let error = credential(
        &store,
        &json!({"id":"job","providerId":"fal","endpoint":"fal-ai/test"}),
    )
    .unwrap_err();
    let issue = error.downcast_ref::<crate::app_error::AppError>().unwrap();
    assert_eq!(issue.code, "VALIDATION_FAILED");
    assert!(!issue.retryable);
    assert!(!issue.to_string().contains("never-guess-this-key"));
    let missing_codex = credential(&store, &json!({"providerId":"codex-image"})).unwrap_err();
    assert_eq!(
        missing_codex
            .downcast_ref::<crate::app_error::AppError>()
            .unwrap()
            .code,
        "VALIDATION_FAILED"
    );
    store
        .conn()
        .unwrap()
        .execute(
            "INSERT INTO service_connections VALUES('codex','Codex','codex','codex://local','')",
            [],
        )
        .unwrap();
    assert_eq!(credential(&store, &json!({"providerId":"codex-image","connectionId":"codex","connectionEndpoint":"codex://local"})).unwrap(), "");
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
