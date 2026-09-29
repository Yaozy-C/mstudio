use super::*;
#[test]
fn database_migration_is_backed_up_atomic_and_idempotent() {
    let root = std::env::temp_dir().join(format!("mstudio-direct-{}", mstudio::media::id()));
    std::fs::create_dir_all(&root).unwrap();
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT); CREATE TABLE projects(id TEXT PRIMARY KEY,name TEXT,document TEXT,updated INTEGER); CREATE TABLE jobs(id TEXT PRIMARY KEY,project_id TEXT,data TEXT); CREATE TABLE skill_resources(skill_id TEXT,path TEXT,text TEXT,revision INTEGER,updated INTEGER);").unwrap();
    crate::project_storage::init(&db).unwrap();
    let original = json!({"id":"p","assets":[{"id":"photo","path":"/photo.png"}],"sharedAssets":[{"id":"person","name":"Person","views":[{"id":"front","name":"Front","assetId":"photo"}]}],"nodes":[{"id":"a","sharedReferences":[{"sharedAssetId":"person","viewId":"front","purpose":"identity"}]},{"id":"b","sharedReferences":[{"sharedAssetId":"person","viewId":"front","purpose":"wardrobe"}]}]});
    db.execute(
        "INSERT INTO projects VALUES('p','P',?1,7)",
        [original.to_string()],
    )
    .unwrap();
    db.execute("INSERT INTO jobs VALUES('j','p',?1)", [json!({"source":{"task":{"assetTarget":{"sharedAssetId":"person","viewId":"front"},"inputs":[{"assetId":"historical"}],"prompt":"keep me"}}}).to_string()]).unwrap();
    db.execute("INSERT INTO settings VALUES('agents',?1)", [json!([{"id":"custom","instructions":"Custom prefix. Shared subjects and views are referenced through sharedReferences. Custom suffix.","revision":9}]).to_string()]).unwrap();
    db.execute("INSERT INTO skill_resources VALUES('custom','CORE.md','Use shared_assets to inventory recurring subjects and their views.',2,0)", []).unwrap();
    migrate(&db, &root).unwrap();
    let saved: String = db
        .query_row("SELECT document FROM projects", [], |r| r.get(0))
        .unwrap();
    let p: Value = serde_json::from_str(&saved).unwrap();
    assert!(p.get("sharedAssets").is_none());
    assert_eq!(p["nodes"][0]["references"][0]["assetId"], "photo");
    assert_eq!(p["nodes"][1]["references"][0]["assetId"], "photo");
    let job: String = db
        .query_row("SELECT data FROM jobs", [], |r| r.get(0))
        .unwrap();
    assert!(!job.contains("assetTarget"));
    assert!(job.contains("historical"));
    assert!(job.contains("keep me"));
    let agents: String = db
        .query_row("SELECT value FROM settings WHERE key='agents'", [], |r| {
            r.get(0)
        })
        .unwrap();
    assert!(!agents.contains("sharedReferences"));
    assert!(agents.contains("Custom prefix"));
    assert!(agents.contains("Custom suffix"));
    let rule: String = db
        .query_row("SELECT text FROM skill_resources", [], |r| r.get(0))
        .unwrap();
    assert!(!rule.contains("shared_assets"));
    let backup: String = db
        .query_row(
            "SELECT value FROM settings WHERE key='direct_image_references_v1'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    let before = Connection::open(backup).unwrap();
    let raw: String = before
        .query_row("SELECT document FROM projects", [], |r| r.get(0))
        .unwrap();
    assert_eq!(serde_json::from_str::<Value>(&raw).unwrap(), original);
    migrate(&db, &root).unwrap();
    assert_eq!(
        db.query_row("SELECT document FROM projects", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        saved
    );
    assert_eq!(std::fs::read_dir(&root).unwrap().count(), 1);
    drop(before);
    drop(db);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn flatten_preserves_direct_refs_history_files_and_missing_view_intent() {
    let mut p = json!({
        "sharedAssets":[{"id":"person","name":"Person","views":[{"id":"front","name":"Front","assetId":"photo"},{"id":"side","name":"Side"}]}],
        "assets":[{"id":"photo","path":"/photo.png","generated":true}],
        "nodes":[{"id":"shot","text":"Action","references":[{"assetId":"photo","purpose":"user purpose"}],"sharedReferences":[{"sharedAssetId":"person","viewId":"front","purpose":"identity"},{"sharedAssetId":"person","viewId":"side","purpose":"side"}]}],
        "production":{"drafts":{"old":{"assetTarget":{"sharedAssetId":"person","viewId":"front"},"inputs":[{"assetId":"old-photo","purpose":"original"}],"prompt":"original prompt","resultAssetId":"photo","status":"COMPLETED"}}}
    });
    document(&mut p);
    assert!(p.get("sharedAssets").is_none());
    assert!(p["nodes"][0].get("sharedReferences").is_none());
    assert_eq!(
        p["nodes"][0]["references"],
        json!([{"assetId":"photo","purpose":"user purpose"}])
    );
    assert!(
        p["nodes"][0]["text"]
            .as_str()
            .unwrap()
            .contains("Person · Side")
    );
    assert_eq!(p["assets"][0]["path"], "/photo.png");
    assert_eq!(p["assets"][0]["inLibrary"], true);
    assert_eq!(
        p["production"]["drafts"]["old"]["inputs"],
        json!([{"assetId":"old-photo","purpose":"original"}])
    );
    assert_eq!(
        p["production"]["drafts"]["old"]["prompt"],
        "original prompt"
    );
    assert!(
        p["production"]["drafts"]["old"]
            .get("assetTarget")
            .is_none()
    );
    let again = p.clone();
    document(&mut p);
    assert_eq!(p, again);
}
