//! Explicit opt-in audit against a disposable SQLite backup, never the running DB.
use super::*;
use sha2::{Digest, Sha256};

#[test]
#[ignore = "requires MSTUDIO_DATABASE_AUDIT pointing to a disposable database copy"]
fn existing_database_round_trip() {
    let path = std::env::var("MSTUDIO_DATABASE_AUDIT").expect("disposable database copy path");
    let db = Connection::open(&path).unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON; PRAGMA journal_mode=WAL;")
        .unwrap();
    schema::init(&db).unwrap();
    media_store::init(&db).unwrap();
    let mut originals = Vec::new();
    let mut before = 0usize;
    for (table, column) in [("agent_events", "payload"), ("jobs", "data")] {
        let mut stmt = db
            .prepare(&format!(
                "SELECT rowid,{column} FROM {table} ORDER BY rowid"
            ))
            .unwrap();
        let mut rows = stmt.query([]).unwrap();
        while let Some(row) = rows.next().unwrap() {
            let id: i64 = row.get(0).unwrap();
            let raw: String = row.get(1).unwrap();
            before += raw.len();
            let value: Value = serde_json::from_str(&raw).unwrap();
            originals.push((
                table,
                column,
                id,
                Sha256::digest(value.to_string().as_bytes()),
            ));
        }
    }
    schema::migrate(&db).unwrap();
    let mut after = 0usize;
    for (table, column, id, digest) in &originals {
        let raw: String = db
            .query_row(
                &format!("SELECT {column} FROM {table} WHERE rowid=?1"),
                [id],
                |r| r.get(0),
            )
            .unwrap();
        after += raw.len();
        let value = if *table == "jobs" {
            let job: String = db
                .query_row("SELECT id FROM jobs WHERE rowid=?1", [id], |r| r.get(0))
                .unwrap();
            blobs::hydrate(
                &db,
                blobs::Owner::Job(&job),
                serde_json::from_str(&raw).unwrap(),
            )
            .unwrap()
        } else {
            blobs::event(&db, *id, &raw).unwrap()
        };
        assert_eq!(
            &Sha256::digest(value.to_string().as_bytes()),
            digest,
            "{table} row {id}"
        );
    }
    let integrity: String = db
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .unwrap();
    assert_eq!(integrity, "ok");
    assert_eq!(
        db.query_row("SELECT count(*) FROM pragma_foreign_key_check", [], |r| r
            .get::<_, i64>(
            0
        ))
        .unwrap(),
        0
    );
    let (count, bytes): (i64, i64) = db
        .query_row(
            "SELECT count(*),sum(length(data)) FROM content_blobs",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    println!(
        "{}",
        json!({"verifiedRows":originals.len(),"beforeJsonBytes":before,"afterJsonBytes":after,"blobs":count,"blobBytes":bytes,"databaseBytes":std::fs::metadata(path).unwrap().len()})
    );
}
