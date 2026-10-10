use super::*;

#[test]
fn previous_default_upgrades_instructions_before_remapping_its_binding() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    super::super::storage::init(&db).unwrap();
    let current = crate::assistant::profiles::builtins()
        .into_iter()
        .find(|p| p.id == "image")
        .unwrap();
    let mut old = current.clone();
    old.skill_ids = vec!["image-production".into()];
    old.instructions = r#"Own still-image execution only: select the requested visible moment, write image prompts, assign reference purposes, prepare needed identity assets, generate and repair images, and inspect actual pixels. Read image-production; use its local camera and material methods only for this still. Distinguish new scenes, local edits, reusable assets, covers and control frames. Preserve approved composition and product evidence; do not redesign the script or shot plan. Save framePrompt and real image asset/frame links. Do not write video prompts, generate video or edit timelines. Report static evidence separately from unverified motion or sound."#.into();
    db.execute(
        "INSERT INTO settings VALUES('agents',?1)",
        [serde_json::to_string(&vec![old]).unwrap()],
    )
    .unwrap();
    migrate(
        &db,
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills"),
    )
    .unwrap();
    let raw: String = db
        .query_row("SELECT value FROM settings WHERE key='agents'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let agents: Vec<crate::assistant::profiles::AgentProfile> = serde_json::from_str(&raw).unwrap();
    assert_eq!(agents[0].skill_ids, current.skill_ids);
    assert_eq!(agents[0].instructions, current.instructions);
}

#[test]
fn installs_boards_preserves_legacy_rules_and_unmounts_them_once() {
    let db = Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    super::super::storage::init(&db).unwrap();
    db.execute("INSERT INTO skill_resources(skill_id,path,text,revision) VALUES('image-production','SKILL.md','MY OLD RULES',8),('product-video-production','SKILL.md','MY OLD VIDEO RULES',4)", []).unwrap();
    let mut profile = crate::assistant::profiles::builtins()
        .into_iter()
        .find(|p| p.id == "image")
        .unwrap();
    profile.skill_ids = vec![
        "image-production".into(),
        "storyboard-image-production".into(),
    ];
    profile.instructions = "CUSTOM PREFIX. Read image-production; CUSTOM SUFFIX".into();
    profile.enabled = false;
    let old = serde_json::to_string(&vec![profile.clone()]).unwrap();
    db.execute("INSERT INTO settings VALUES('agents',?1)", [&old])
        .unwrap();
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills");
    migrate(&db, &root).unwrap();
    let legacy: (String, i64) = db
        .query_row(
            "SELECT text,revision FROM skill_resources WHERE skill_id='image-production'",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .unwrap();
    assert_eq!(legacy, ("MY OLD RULES".into(), 8));
    let setting: String = db
        .query_row("SELECT value FROM settings WHERE key='agents'", [], |r| {
            r.get(0)
        })
        .unwrap();
    let agents: Vec<crate::assistant::profiles::AgentProfile> =
        serde_json::from_str(&setting).unwrap();
    assert_eq!(agents[0].skill_ids, [PACKAGES[0]]);
    assert_eq!(agents[0].tool_ids, profile.tool_ids);
    assert!(!agents[0].enabled);
    assert!(agents[0].instructions.starts_with("CUSTOM PREFIX"));
    let scope = crate::assistant::profiles::skill_setting(&agents[0]);
    assert!(
        super::super::storage::read(&db, &scope, "image-production", "SKILL.md", 0, true).is_err()
    );
    for id in PACKAGES {
        assert!(super::super::storage::read(&db, "", id, "SKILL.md", 0, true).is_ok());
    }
    let backup: String = db
        .query_row(
            "SELECT value FROM settings WHERE key=?1",
            [format!("{MARKER}_agents_backup")],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(backup, old);
    db.execute(
        "UPDATE skill_resources SET text='CUSTOM NEW BOARD' WHERE skill_id=?1 AND path='CORE.md'",
        [PACKAGES[0]],
    )
    .unwrap();
    migrate(&db, Path::new("/unavailable")).unwrap();
    let text: String = db
        .query_row(
            "SELECT text FROM skill_resources WHERE skill_id=?1 AND path='CORE.md'",
            [PACKAGES[0]],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(text, "CUSTOM NEW BOARD");
}
