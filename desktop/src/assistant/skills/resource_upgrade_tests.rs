use super::*;
use serde_json::json;

fn setup() -> (Connection, std::path::PathBuf) {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    super::super::storage::init(&db).unwrap();
    let root =
        std::env::temp_dir().join(format!("mstudio-resource-upgrade-{}", mstudio::media::id()));
    std::fs::create_dir_all(root.join("test")).unwrap();
    (db, root)
}
fn manifest() -> Manifest {
    serde_json::from_value(json!({"agents":{},"resources":[
        {"skill":"test","path":"SKILL.md","previous":digest("old default"),"remove":false},
        {"skill":"test","path":"optional.md","previous":null,"remove":false},
        {"skill":"test","path":"duplicate.md","previous":digest("old duplicate"),"remove":true}
    ]}))
    .unwrap()
}
fn text(db: &Connection, path: &str) -> Option<String> {
    db.query_row(
        "SELECT text FROM skill_resources WHERE skill_id='test' AND path=?1",
        [path],
        |r| r.get(0),
    )
    .optional()
    .unwrap()
}
#[test]
fn upgrades_known_defaults_removes_duplicates_and_preserves_custom_rules_once() {
    let (db, root) = setup();
    std::fs::write(root.join("test/SKILL.md"), "new concise default").unwrap();
    std::fs::write(root.join("test/optional.md"), "new optional guide").unwrap();
    db.execute_batch("INSERT INTO skill_resources(skill_id,path,text,revision) VALUES('test','SKILL.md','old default',7),('test','duplicate.md','old duplicate',3),('test','optional.md','user owned optional',9);").unwrap();
    let mut agents = super::super::super::profiles::builtins();
    let editor = agents.iter_mut().find(|p| p.id == "editor").unwrap();
    editor.instructions = format!("CUSTOM PREFIX {OLD_EDITOR_CHECK} CUSTOM SUFFIX");
    editor.name = "My editor".into();
    editor.enabled = false;
    let revision = editor.revision;
    db.execute(
        "INSERT INTO settings VALUES('agents',?1)",
        [serde_json::to_string(&agents).unwrap()],
    )
    .unwrap();
    apply(&db, &root, &manifest()).unwrap();
    assert_eq!(text(&db, "SKILL.md").unwrap(), "new concise default");
    assert!(text(&db, "duplicate.md").is_none());
    assert_eq!(text(&db, "optional.md").unwrap(), "user owned optional");
    let saved = super::super::super::profiles::read(&db).unwrap();
    let editor = saved.iter().find(|p| p.id == "editor").unwrap();
    assert_eq!(
        editor.instructions,
        format!("CUSTOM PREFIX {EDITOR_CHECK} CUSTOM SUFFIX")
    );
    assert_eq!(editor.name, "My editor");
    assert!(!editor.enabled);
    assert_eq!(editor.revision, revision + 1);
    apply(&db, &root, &manifest()).unwrap();
    assert_eq!(
        db.query_row(
            "SELECT revision FROM skill_resources WHERE path='SKILL.md'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        8
    );
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn failed_upgrade_rolls_back_and_modified_resources_are_preserved() {
    let (db, root) = setup();
    std::fs::write(root.join("test/SKILL.md"), "new default").unwrap();
    db.execute_batch("INSERT INTO skill_resources(skill_id,path,text) VALUES('test','SKILL.md','old default'),('test','duplicate.md','user changed duplicate');").unwrap();
    assert!(apply(&db, &root, &manifest()).is_err());
    assert_eq!(text(&db, "SKILL.md").unwrap(), "old default");
    assert_eq!(
        db.query_row(
            "SELECT count(*) FROM settings WHERE key=?1",
            [MARKER],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        0
    );
    std::fs::write(root.join("test/optional.md"), "optional detail").unwrap();
    db.execute(
        "UPDATE skill_resources SET text='custom main' WHERE path='SKILL.md'",
        [],
    )
    .unwrap();
    apply(&db, &root, &manifest()).unwrap();
    assert_eq!(text(&db, "SKILL.md").unwrap(), "custom main");
    assert_eq!(text(&db, "duplicate.md").unwrap(), "user changed duplicate");
    assert_eq!(text(&db, "optional.md").unwrap(), "optional detail");
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn fresh_install_keeps_current_resources_through_all_prior_migrations() {
    let (db, temporary) = setup();
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills");
    super::super::storage::seed(&db, &root).unwrap();
    super::super::storage::install_asset_defaults(&db).unwrap();
    super::super::storage::install_core_defaults(&db).unwrap();
    super::super::prompt_migration::migrate(&db).unwrap();
    super::super::prompt_scope::install(&db).unwrap();
    super::super::contract_migration::migrate(&db).unwrap();
    migrate(&db, &root).unwrap();
    let mut stmt = db
        .prepare("SELECT skill_id,path,text,revision FROM skill_resources")
        .unwrap();
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, i64>(3)?,
            ))
        })
        .unwrap();
    for row in rows {
        let (skill, path, text, revision) = row.unwrap();
        assert_eq!(
            text,
            std::fs::read_to_string(root.join(skill).join(path)).unwrap()
        );
        assert_eq!(revision, 1);
    }
    std::fs::remove_dir_all(temporary).unwrap();
}
