use super::*;
use serde_json::json;

fn database() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch(
        "CREATE TABLE assets(data TEXT); CREATE TABLE jobs(data TEXT);
         CREATE TABLE agent_messages(payload TEXT); CREATE TABLE previs_exports(data TEXT);
         CREATE TABLE previs_media(data TEXT);",
    )
    .unwrap();
    db
}

#[test]
fn referenced_directories_survive_json_escaping_in_every_snapshot_source() {
    for (table, column) in [
        ("assets", "data"),
        ("jobs", "data"),
        ("agent_messages", "payload"),
        ("previs_exports", "data"),
        ("previs_media", "data"),
    ] {
        for (prefix, separator) in [
            (r"D:\media\exports\previs-first", "\\"),
            ("/media/exports/previs-\"first\"", "/"),
        ] {
            let db = database();
            let snapshot = json!({"inputs":[{"media":{"path":format!("{prefix}{separator}media{separator}video.mp4")}}]});
            db.execute(
                &format!("INSERT INTO {table} VALUES(?1)"),
                [snapshot.to_string()],
            )
            .unwrap();
            assert!(protected(&db, prefix).unwrap(), "{table}: {prefix}");
            assert!(!protected(&db, &format!("{prefix}-unused")).unwrap());
            // A different render whose name starts with the same text is not a reference.
            assert!(!protected(&db, prefix.rsplit_once('-').unwrap().0).unwrap());
            assert!(
                db.execute(&format!("UPDATE {table} SET {column}='{{'"), [])
                    .is_ok()
            );
            assert!(
                protected(&db, prefix).is_err(),
                "unreadable snapshots must report failure before deletion"
            );
        }
    }
}
