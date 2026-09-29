use super::{profiles, skills};
use std::{fs, path::Path};

fn copy_tree(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for entry in fs::read_dir(from).unwrap() {
        let entry = entry.unwrap();
        let target = to.join(entry.file_name());
        if entry.file_type().unwrap().is_dir() {
            copy_tree(&entry.path(), &target);
        } else {
            fs::copy(entry.path(), target).unwrap();
        }
    }
}

#[test]
fn shipped_rules_work_from_a_relocated_resource_directory() {
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills");
    let temp = std::env::temp_dir().join(format!("mstudio-bundled-{}", mstudio::media::id()));
    copy_tree(&repo, &temp.join("skills"));
    let root = skills::directory(&temp).unwrap();
    let db = rusqlite::Connection::open_in_memory().unwrap();
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
        .unwrap();
    skills::storage::seed(&db, &root).unwrap();
    let catalog = skills::storage::catalog(&db, "").unwrap();
    assert_eq!(catalog.as_array().unwrap().len(), profiles::SKILL_IDS.len());
    for id in profiles::SKILL_IDS {
        let page = skills::storage::read(&db, "", id, "SKILL.md", 0, true).unwrap();
        assert!(page["text"].as_str().unwrap().contains(id));
        assert!(page["totalCharacters"].as_u64().unwrap() > 0);
    }
    let path = "../creative-ad-director/references/video-prompt-writing.md";
    let enabled = "[\"skill-product-video-production\"]";
    let page =
        skills::storage::read(&db, enabled, "product-video-production", path, 0, true).unwrap();
    assert_eq!(page["skill"], "creative-ad-director");
    let source =
        fs::read_to_string(root.join("creative-ad-director/references/video-prompt-writing.md"))
            .unwrap();
    let text = page["text"].as_str().unwrap();
    assert!(!text.is_empty() && source.starts_with(text));
    // A packaged install must never fall back to a developer's repository or home directory.
    fs::remove_dir_all(root).unwrap();
    assert!(skills::directory(&temp).is_err());
    fs::remove_dir_all(temp).unwrap();
}

#[test]
fn bundle_manifest_includes_the_repository_skill_tree() {
    let config: serde_json::Value =
        serde_json::from_str(include_str!("../../tauri.conf.json")).unwrap();
    assert_eq!(config["bundle"]["resources"]["../skills/"], "skills/");
}
