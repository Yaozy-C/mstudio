use super::*;
use crate::assistant::journal;
use base64::{Engine, engine::general_purpose::STANDARD};

pub const PNG: &str =
    "iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAQAAAC1HAwCAAAAC0lEQVR42mP8/x8AAwMCAO+jv1sAAAAASUVORK5CYII=";
fn store() -> Store {
    let store = Store::open(
        std::env::temp_dir().join(format!("mstudio-media-refs-{}", mstudio::media::id())),
    )
    .unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute_batch("INSERT INTO projects VALUES('p','p','{}',0),('q','q','{}',0);")
        .unwrap();
    store
}
#[test]
fn binary_media_is_local_shared_and_restores_exact_provider_input() {
    let store = store();
    let url = format!("data:image/png;base64,{PNG}");
    let event = json!({"image":{"data":{"type":"base64","value":PNG}},"literal":{"media_digest":"untrusted"}});
    journal::append(&store, "p", "t", "tool/result", event.clone()).unwrap();
    let job = json!({"id":"j","projectId":"q","status":"COMPLETED","input":{"image":url}});
    crate::jobs::save(&store, &job).unwrap();
    {
        let db = store.db.lock().unwrap();
        assert_eq!(
            db.query_row("SELECT count(*) FROM stored_media", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            1
        );
        assert_eq!(
            db.query_row(
                "SELECT sum(length(data)) FROM content_blobs WHERE media_digest IS NOT NULL",
                [],
                |r| r.get::<_, i64>(0)
            )
            .unwrap(),
            0
        );
        let (seq, raw): (i64, String) = db
            .query_row("SELECT seq,payload FROM agent_events", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        assert!(!raw.contains(PNG));
        assert_eq!(blobs::event(&db, seq, &raw).unwrap(), event);
    }
    let files = std::fs::read_dir(store.media_root().join("reference-media"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .collect::<Vec<_>>();
    assert_eq!(files.len(), 1);
    assert_eq!(
        std::fs::read(&files[0]).unwrap(),
        STANDARD.decode(PNG).unwrap()
    );
    assert_eq!(crate::jobs::get(&store, "j").unwrap(), job);
    crate::project_storage::remove(&store, "p").unwrap();
    assert!(files[0].exists());
    assert_eq!(crate::jobs::get(&store, "j").unwrap(), job);
    crate::project_storage::remove(&store, "q").unwrap();
    assert!(!files[0].exists());
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn file_write_rollback_is_reclaimed_and_missing_media_fails_explicitly() {
    let store = store();
    let bytes = STANDARD.decode(PNG).unwrap();
    {
        let db = store.db.lock().unwrap();
        let tx = db.unchecked_transaction().unwrap();
        media_store::save(&tx, &bytes).unwrap();
    }
    let root = store.root.clone();
    drop(store);
    let store = Store::open(root.clone()).unwrap();
    assert_eq!(
        std::fs::read_dir(root.join("reference-media"))
            .unwrap()
            .count(),
        0
    );
    let job =
        json!({"id":"j","projectId":"p","input":{"image":format!("data:image/png;base64,{PNG}")}});
    crate::jobs::save(&store, &job).unwrap();
    let file = std::fs::read_dir(root.join("reference-media"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    std::fs::write(&file, b"corrupt").unwrap();
    assert!(crate::jobs::get(&store, "j").is_err());
    std::fs::remove_file(file).unwrap();
    assert!(crate::jobs::get(&store, "j").is_err());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
