use super::{profiles, skills};
use rusqlite::{Connection, params};
use serde_json::json;

#[test]
fn role_catalog_does_not_eagerly_load_detailed_rule_bodies() {
    let db = Connection::open_in_memory().unwrap();
    skills::storage::init(&db).unwrap();
    let rules = "UNLOADED_SKILL_BODY".repeat(6000);
    for id in profiles::SKILL_IDS {
        db.execute(
            "INSERT INTO skill_resources(skill_id,path,text) VALUES(?1,'SKILL.md',?2)",
            params![id, rules],
        )
        .unwrap();
    }
    for profile in profiles::builtins() {
        let catalog = skills::storage::catalog(&db, &profiles::skill_setting(&profile)).unwrap();
        let snapshot = json!({"skills":catalog,"agent":{
            "name":profile.name,"instructions":profile.instructions,
            "skills":profile.skill_ids,"tools":profile.tool_ids,
            "canEdit":profiles::allows(&profile,"edit")
        }});
        let system = super::prompts::system(&snapshot);
        assert!(system.contains("mstudio_read_skill"));
        assert!(!system.contains("UNLOADED_SKILL_BODY"));
        assert!(!snapshot.to_string().contains("UNLOADED_SKILL_BODY"));
    }
    let page = skills::storage::read(&db, "", "product-storyboard", "SKILL.md", 0, true).unwrap();
    assert!(
        page["text"]
            .as_str()
            .unwrap()
            .contains("UNLOADED_SKILL_BODY")
    );
    skills::storage::save(
        &db,
        "product-storyboard",
        "SKILL.md",
        "Updated current instructions",
        1,
    )
    .unwrap();
    assert_eq!(
        skills::storage::read(&db, "", "product-storyboard", "SKILL.md", 0, true).unwrap()["text"],
        "Updated current instructions"
    );
    assert!(skills::guidance(&skills::storage::catalog(&db, "[]").unwrap()).contains("Skills: []"));
}
