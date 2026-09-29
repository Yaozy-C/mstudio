use super::*;

#[test]
#[ignore = "requires MSTUDIO_MEDIA_AUDIT pointing to a disposable database copy"]
fn migrate_existing_media_to_disposable_local_files() {
    let path = PathBuf::from(std::env::var("MSTUDIO_MEDIA_AUDIT").unwrap());
    let db = Connection::open(&path).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON;").unwrap();
    let root = path.parent().unwrap().join("local-media");
    std::fs::create_dir_all(&root).unwrap();
    db.execute("INSERT INTO settings(key,value) VALUES('file-storage',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[json!({"directory":root,"previous":[]}).to_string()]).unwrap();
    media_store::init(&db).unwrap();
    media_migration::migrate(&db).unwrap();
    media_migration::migrate(&db).unwrap();
    let (files, bytes): (i64, i64) = db
        .query_row("SELECT count(*),sum(bytes) FROM stored_media", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .unwrap();
    assert_eq!(
        db.query_row(
            "SELECT sum(length(data)) FROM content_blobs WHERE media_digest IS NOT NULL",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    assert_eq!(
        db.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    println!(
        "{}",
        json!({"files":files,"mediaBytes":bytes,"databaseBytes":std::fs::metadata(&path).unwrap().len()})
    );
}
