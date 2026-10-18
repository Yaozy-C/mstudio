use super::{run, tests::Fixture};
use crate::database::blobs::{self, Owner};
use serde_json::{Value, json};
use std::{
    fs,
    time::{Duration, SystemTime},
};

#[test]
fn local_job_media_snapshots_and_original_source_code_survive_collection() {
    let f = Fixture::new();
    let source = f.file("previs/p/model/model.js");
    let texture = f.file("assets/texture-without-asset-record.png");
    let db = f.store.db.lock().unwrap();
    db.execute("INSERT INTO previs_files VALUES('p','model','model',?1)",[
        json!({"id":"model","kind":"model","name":"model","code":format!("const texture = '{}';", texture.display()),"revision":1,"assetIds":[],"bindings":{}}).to_string()
    ]).unwrap();
    let mut value = json!({"status":"COMPLETED","input":{"image":"data:image/png;base64,YQ=="}});
    db.execute("INSERT INTO jobs VALUES('j','p',?1)", [value.to_string()])
        .unwrap();
    blobs::pack(&db, Owner::Job("j"), &mut value).unwrap();
    db.execute("UPDATE jobs SET data=?1 WHERE id='j'", [value.to_string()])
        .unwrap();
    let digest: String = db
        .query_row(
            "SELECT media_digest FROM content_blobs WHERE media_digest IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let snapshot = f.root.join("reference-media").join(&digest);
    fs::File::options()
        .write(true)
        .open(&snapshot)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(600)))
        .unwrap();
    drop(db);
    assert_eq!(f.clean().deleted_files, 0);
    assert!(source.exists());
    assert!(texture.exists());
    assert!(snapshot.exists());
    let db = f.store.db.lock().unwrap();
    let raw: String = db
        .query_row("SELECT data FROM jobs WHERE id='j'", [], |r| r.get(0))
        .unwrap();
    let hydrated =
        blobs::hydrate(&db, Owner::Job("j"), serde_json::from_str(&raw).unwrap()).unwrap();
    assert_eq!(hydrated["input"]["image"], "data:image/png;base64,YQ==");
}
#[test]
fn detached_blob_records_are_counted_and_reclaimed_without_touching_live_history() {
    let f = Fixture::new();
    let db = f.store.db.lock().unwrap();
    let mut value = json!({"status":"COMPLETED","input":{"image":"data:image/png;base64,YQ=="}});
    db.execute("INSERT INTO jobs VALUES('j','p',?1)", [value.to_string()])
        .unwrap();
    blobs::pack(&db, Owner::Job("j"), &mut value).unwrap();
    let digest: String = db
        .query_row(
            "SELECT media_digest FROM content_blobs WHERE media_digest IS NOT NULL",
            [],
            |r| r.get(0),
        )
        .unwrap();
    db.execute("DELETE FROM jobs WHERE id='j'", []).unwrap();
    drop(db);
    let path = f.root.join("reference-media").join(digest);
    fs::File::options()
        .write(true)
        .open(&path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(SystemTime::now() - Duration::from_secs(600)))
        .unwrap();
    let report = f.scan().report;
    assert_eq!(report.unused_records, 2);
    assert_eq!(report.unused_files, 1);
    let result = run::clean(&f.store, &f.data, &report.fingerprint).unwrap();
    assert_eq!(result.deleted_records, 2);
    assert_eq!(result.deleted_files, 1);
    assert!(!path.exists());
    assert_eq!(f.scan().report.unused_records, 0);
}
#[test]
fn metadata_without_a_canonical_row_and_shared_paths_are_still_valid_roots() {
    let f = Fixture::new();
    let path = f.file("assets/source.png");
    let asset = json!({"id":"only-in-doc","name":"original","path":path,"generated":false});
    f.document(vec![asset], json!([]));
    f.clean();
    assert!(path.exists());
    let db = f.store.db.lock().unwrap();
    let doc: Value = serde_json::from_str(
        &db.query_row::<String, _, _>("SELECT document FROM projects WHERE id='p'", [], |r| {
            r.get(0)
        })
        .unwrap(),
    )
    .unwrap();
    assert_eq!(doc["assets"].as_array().unwrap().len(), 1);
}
#[cfg(unix)]
#[test]
fn a_symlink_swap_after_review_cannot_delete_an_outside_file() {
    use std::os::unix::fs::symlink;
    let f = Fixture::new();
    let target = f.file("assets/orphan.png");
    let reviewed = f.scan().report.fingerprint;
    fs::remove_file(&target).unwrap();
    let outside = f.base.join("outside");
    fs::write(&outside, "keep me").unwrap();
    symlink(&outside, &target).unwrap();
    assert!(run::clean(&f.store, &f.data, &reviewed).is_err());
    assert_eq!(fs::read_to_string(&outside).unwrap(), "keep me");
}

#[test]
fn recent_import_metadata_is_kept_until_the_file_grace_period_expires() {
    let f = Fixture::new();
    let fresh = f.asset("just-imported", json!({}));
    let path = fresh["path"].as_str().unwrap();
    fs::File::options()
        .write(true)
        .open(path)
        .unwrap()
        .set_times(fs::FileTimes::new().set_modified(SystemTime::now()))
        .unwrap();
    f.asset("old-orphan", json!({}));
    let report = f.clean();
    assert_eq!(report.deleted_records, 1);
    assert!(std::path::Path::new(path).exists());
    assert_eq!(
        f.store
            .db
            .lock()
            .unwrap()
            .query_row::<u64, _, _>(
                "SELECT count(*) FROM assets WHERE id='just-imported'",
                [],
                |r| r.get(0)
            )
            .unwrap(),
        1
    );
}
