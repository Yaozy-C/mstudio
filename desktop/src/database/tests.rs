use super::*;
use blobs::Owner;

fn db() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON;
        CREATE TABLE projects(id TEXT PRIMARY KEY);
        CREATE TABLE pending_file_deletions(project_id TEXT,path TEXT,PRIMARY KEY(project_id,path));
        CREATE TABLE jobs(id TEXT PRIMARY KEY,project_id TEXT,data TEXT);
        CREATE TABLE agent_events(seq INTEGER PRIMARY KEY,project_id TEXT,turn_id TEXT,kind TEXT,payload TEXT);
        CREATE TABLE agent_messages(project_id TEXT,attribution TEXT,role TEXT);
        CREATE TABLE project_assets(project_id TEXT,asset_id TEXT);
        CREATE TABLE project_files(project_id TEXT,path TEXT);
        CREATE TABLE subagent_inbox(seq INTEGER PRIMARY KEY,child_id TEXT,consumed INTEGER);
        CREATE TABLE subagent_runs(project_id TEXT,last_turn TEXT);
        INSERT INTO projects VALUES('p');").unwrap();
    schema::init(&db).unwrap();
    media_store::init(&db).unwrap();
    db
}
#[test]
fn migration_is_lossless_resumable_and_deduplicates_across_owners() {
    let db = db();
    let large = "图片 / ~ 🐈".repeat(2000);
    let event = json!({"a/b~c":[large],"callId":"call","userMarker":{"blob":"not-a-reference"}});
    let job = json!({"input":{"image":large},"status":"COMPLETED","shot":{"prompt":large}});
    db.execute(
        "INSERT INTO agent_events VALUES(1,'p','t','tool/result',?1)",
        [event.to_string()],
    )
    .unwrap();
    db.execute("INSERT INTO jobs VALUES('j','p',?1)", [job.to_string()])
        .unwrap();
    // Simulate an interruption after one parent was packed, before the version commit.
    let tx = db.unchecked_transaction().unwrap();
    let mut packed = event.clone();
    blobs::pack(&tx, Owner::Event(1), &mut packed).unwrap();
    tx.execute("UPDATE agent_events SET payload=?1", [packed.to_string()])
        .unwrap();
    tx.commit().unwrap();
    schema::migrate(&db).unwrap();
    schema::migrate(&db).unwrap();
    let raw: String = db
        .query_row("SELECT payload FROM agent_events", [], |r| r.get(0))
        .unwrap();
    assert_eq!(blobs::event(&db, 1, &raw).unwrap(), event);
    let raw: String = db
        .query_row("SELECT data FROM jobs", [], |r| r.get(0))
        .unwrap();
    assert_eq!(
        blobs::hydrate(&db, Owner::Job("j"), serde_json::from_str(&raw).unwrap()).unwrap(),
        job
    );
    assert_eq!(
        db.query_row("SELECT count(*) FROM content_blobs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(raw.contains("图片")); // Summary fields are not replaced by references.
    db.execute("DELETE FROM agent_events", []).unwrap();
    blobs::collect(&db).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM content_blobs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
    db.execute("DELETE FROM jobs", []).unwrap();
    blobs::collect(&db).unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM content_blobs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn writes_roll_back_and_projections_do_not_load_omitted_content() {
    let db = db();
    db.execute_batch("INSERT INTO agent_events VALUES(1,'p','t','tool/result','{}');")
        .unwrap();
    let mut value = json!({"value":"x".repeat(8000),"callId":"call"});
    {
        let tx = db.unchecked_transaction().unwrap();
        blobs::pack(&tx, Owner::Event(1), &mut value).unwrap();
    }
    assert_eq!(
        db.query_row("SELECT count(*) FROM content_blobs", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    value = json!({"value":"x".repeat(8000),"callId":"call"});
    blobs::pack(&db, Owner::Event(1), &mut value).unwrap();
    assert_eq!(
        blobs::event(&db, 1, r#"{"callId":"call"}"#).unwrap(),
        json!({"callId":"call"})
    );
    db.execute("UPDATE content_blobs SET data=x'0000'", [])
        .unwrap();
    assert!(blobs::hydrate(&db, Owner::Event(1), value).is_err());
}
#[test]
fn hot_queries_use_matching_indexes() {
    let db = db();
    for (query, index) in [
        (
            "SELECT payload FROM agent_events WHERE project_id='p' AND turn_id='t' ORDER BY seq",
            "agent_events_turn",
        ),
        (
            "SELECT payload FROM agent_events WHERE project_id='p' AND turn_id='t' AND kind='tool/result' AND json_extract(payload,'$.callId')='c' ORDER BY seq DESC LIMIT 1",
            "agent_events_call",
        ),
        (
            "SELECT data FROM jobs WHERE project_id='p' ORDER BY rowid DESC",
            "jobs_project",
        ),
        (
            "SELECT project_id FROM project_assets WHERE asset_id='a'",
            "project_assets_asset",
        ),
    ] {
        let plan: String = db
            .query_row(&format!("EXPLAIN QUERY PLAN {query}"), [], |r| r.get(3))
            .unwrap();
        assert!(plan.contains(index), "{plan}");
    }
}
