use super::*;

#[test]
fn physical_continuity_upgrade_replaces_defaults_preserves_custom_and_runs_once() {
    check_upgrade(
        include_str!("physical_continuity_migration.json"),
        "image_physical_continuity_v1",
        migrate_physical_continuity,
    );
}

#[test]
fn video_physical_chain_upgrade_replaces_defaults_preserves_custom_and_runs_once() {
    check_upgrade(
        include_str!("video_physical_chain_migration.json"),
        "video_physical_chain_v1",
        migrate_video_physical_chain,
    );
}

#[test]
fn director_state_upgrade_replaces_defaults_preserves_custom_and_runs_once() {
    check_upgrade(
        include_str!("director_physical_state_migration.json"),
        "director_physical_state_v1",
        super::super::director_state::migrate,
    );
}

fn check_upgrade(manifest_json: &str, marker: &str, migrate: fn(&Connection, &Path) -> Result<()>) {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    super::super::storage::init(&db).unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills");
    let mut manifest: Manifest = serde_json::from_str(manifest_json).unwrap();
    for (index, resource) in manifest.updates.iter_mut().enumerate() {
        let old = format!("previous shipped rule {index}");
        resource.before = format!("{:x}", Sha256::digest(old.as_bytes()));
        let stored = if index == 0 { "custom user rule" } else { &old };
        db.execute(
            "INSERT INTO skill_resources(skill_id,path,text) VALUES(?1,?2,?3)",
            params![resource.skill, resource.path, stored],
        )
        .unwrap();
    }
    let resources: Vec<_> = manifest
        .updates
        .iter()
        .map(|r| (r.skill.clone(), r.path.clone()))
        .collect();
    apply(&db, &root, manifest, marker).unwrap();
    for (index, (skill, path)) in resources.iter().enumerate() {
        let (text, revision): (String, i64) = db
            .query_row(
                "SELECT text,revision FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![skill, path],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        let expected = if index == 0 {
            "custom user rule".to_owned()
        } else {
            std::fs::read_to_string(root.join(skill).join(path)).unwrap()
        };
        assert_eq!(text, expected);
        assert_eq!(revision, if index == 0 { 1 } else { 2 });
    }
    let backup: String = db
        .query_row(
            "SELECT value FROM settings WHERE key=?1",
            [format!("{marker}_backup")],
            |r| r.get(0),
        )
        .unwrap();
    let backup: Vec<Value> = serde_json::from_str(&backup).unwrap();
    assert_eq!(backup.len(), resources.len());
    assert_eq!(backup[0]["text"], "custom user rule");
    migrate(&db, Path::new("/unavailable")).unwrap();
}
