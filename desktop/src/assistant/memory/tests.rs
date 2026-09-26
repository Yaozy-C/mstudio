use super::*;
use serde_json::json;
fn db() -> Connection {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("PRAGMA foreign_keys=ON; CREATE TABLE projects(id TEXT PRIMARY KEY); INSERT INTO projects VALUES('one'),('two');").unwrap();
    init(&db).unwrap();
    db
}
#[test]
fn project_memory_is_shared_persistent_and_cascade_deleted() {
    let db = db();
    let backend = SqliteMemory(&db);
    let args = json!({"action":"remember","memoryRevision":0,"title":"画幅","content":"竖屏 9:16","evidence":"竖屏"});
    tool::operate(&db, "one", "turn-1", "这个项目用竖屏", true, &args).unwrap();
    assert_eq!(backend.recall("one").unwrap().entries[0].source, "竖屏");
    assert!(backend.recall("two").unwrap().entries.is_empty());
    assert!(backend.recall("missing").is_err());
    assert_eq!(SqliteMemory(&db).recall("one").unwrap().revision, 1);
    db.execute("DELETE FROM projects WHERE id='one'", [])
        .unwrap();
    assert_eq!(
        db.query_row("SELECT count(*) FROM project_memory", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn stale_writes_unproven_facts_and_read_only_agents_cannot_write() {
    let db = db();
    let args = json!({"action":"remember","memoryRevision":0,"title":"风格","content":"自然","evidence":"自然"});
    assert!(tool::operate(&db, "one", "t", "简洁", true, &args).is_err());
    assert!(tool::operate(&db, "one", "t", "自然", false, &args).is_err());
    tool::operate(&db, "one", "t", "自然", true, &args).unwrap();
    assert_eq!(
        tool::operate(&db, "one", "t", "自然", true, &args).unwrap()["code"],
        "MEMORY_REVISION_CONFLICT"
    );
    let mut memory = SqliteMemory(&db).recall("one").unwrap();
    let id = memory.entries[0].id.clone();
    memory.auto_update = false;
    let memory = SqliteMemory(&db).replace("one", memory).unwrap();
    let update =
        json!({"action":"forget","id":id,"memoryRevision":memory.revision,"evidence":"忘记"});
    assert!(tool::operate(&db, "one", "t", "忘记", true, &update).is_err());
    assert!(tool::operate(&db, "one", "t", "", false, &json!({"action":"list"})).is_ok());
}
#[test]
fn correction_replaces_entry_and_disabled_memory_is_not_recalled() {
    let db = db();
    let backend = SqliteMemory(&db);
    tool::operate(&db,"one","t","用竖屏",true,&json!({"action":"remember","memoryRevision":0,"title":"画幅","content":"竖屏","evidence":"竖屏"})).unwrap();
    let mut memory = backend.recall("one").unwrap();
    tool::operate(&db,"one","t2","改横屏",true,&json!({"action":"remember","id":memory.entries[0].id,"memoryRevision":1,"title":"画幅","content":"横屏","evidence":"横屏"})).unwrap();
    memory = backend.recall("one").unwrap();
    assert_eq!(memory.entries.len(), 1);
    assert_eq!(memory.entries[0].content, "横屏");
    assert!(context(&memory, "画幅").to_string().contains("横屏"));
    memory.enabled = false;
    backend.replace("one", memory.clone()).unwrap();
    assert!(!context(&memory, "画幅").to_string().contains("横屏"));
    assert!(tool::operate(&db, "one", "t", "", true, &json!({"action":"list"})).is_err());
}

#[test]
fn memory_protocol_rejects_project_version_and_invented_ids_without_writes() {
    let db = db();
    let mut args =
        json!({"action":"remember","revision":6,"title":"时长","content":"30秒","evidence":"30秒"});
    let conflict = tool::operate(&db, "one", "t", "制作30秒短片", true, &args).unwrap();
    assert_eq!(conflict["code"], "MEMORY_REVISION_CONFLICT");
    assert_eq!(conflict["memoryRevision"], 0);
    args.as_object_mut().unwrap().remove("revision");
    args["memoryRevision"] = json!(0);
    args["id"] = json!("invented-id");
    assert_eq!(
        tool::operate(&db, "one", "t", "制作30秒短片", true, &args).unwrap()["code"],
        "MEMORY_NOT_FOUND"
    );
    assert!(SqliteMemory(&db).recall("one").unwrap().entries.is_empty());
    args.as_object_mut().unwrap().remove("id");
    let saved = tool::operate(&db, "one", "t", "制作30秒短片", true, &args).unwrap();
    assert_eq!(saved["memoryRevision"], 1);
    assert!(saved["entries"][0]["id"].as_str().is_some());
}

#[test]
fn delegated_paraphrase_is_not_user_evidence() {
    let db = db();
    let args = json!({"action":"remember","memoryRevision":0,"title":"方案选择","content":"选择第二个方案","evidence":"用户已明确选定方案 B"});
    assert!(tool::operate(&db, "one", "child", "第二个", true, &args).is_err());
    assert!(SqliteMemory(&db).recall("one").unwrap().entries.is_empty());
}
