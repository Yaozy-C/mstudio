use super::skills;
use serde_json::json;
use std::fs;

#[test]
fn bound_skills_start_as_metadata_and_only_needed_rules_are_read() {
    let root = std::env::temp_dir().join(format!("mstudio-lazy-skills-{}", mstudio::media::id()));
    // No references exist: starting a role must not require stage files.
    let rules = "UNLOADED_SKILL_BODY".repeat(6000);
    for id in super::profiles::SKILL_IDS {
        fs::create_dir_all(root.join(id)).unwrap();
        fs::write(root.join(id).join("SKILL.md"), &rules).unwrap();
    }
    for profile in super::profiles::builtins() {
        let catalog = skills::catalog(&root, &super::profiles::skill_setting(&profile)).unwrap();
        let snapshot = json!({
            "skills": catalog,
            "agent": {
                "name": profile.name, "instructions": profile.instructions,
                "skills": profile.skill_ids, "tools": profile.tool_ids,
                "canEdit": super::profiles::allows(&profile, "edit")
            }
        });
        let system = super::prompts::system(&snapshot);
        assert!(system.contains("mstudio_read_skill"));
        assert!(!system.contains("UNLOADED_SKILL_BODY"));
        assert!(!snapshot.to_string().contains("UNLOADED_SKILL_BODY"));
    }

    let page = skills::read(&root, "", "product-storyboard", "SKILL.md", 0, true).unwrap();
    assert!(
        page["text"]
            .as_str()
            .unwrap()
            .contains("UNLOADED_SKILL_BODY")
    );
    fs::write(
        root.join("product-storyboard/SKILL.md"),
        "Updated current instructions",
    )
    .unwrap();
    let fresh = skills::read(&root, "", "product-storyboard", "SKILL.md", 0, true).unwrap();
    assert_eq!(fresh["text"], "Updated current instructions");
    fs::remove_dir_all(root).unwrap();
}

#[test]
fn local_skill_rules_are_paginated_follow_links_and_respect_switches() {
    let root = std::env::temp_dir().join(format!("mstudio-skills-{}", mstudio::media::id()));
    let id = "product-storyboard";
    fs::create_dir_all(root.join(id).join("references")).unwrap();
    fs::create_dir_all(root.join("product-video-production")).unwrap();
    let rules = "分镜与原始素材规则\n".repeat(700);
    fs::write(root.join(id).join("SKILL.md"), &rules).unwrap();
    fs::write(
        root.join(id).join("references/actions.md"),
        "动作必须有起止状态",
    )
    .unwrap();
    fs::write(
        root.join("product-video-production/SKILL.md"),
        "只修改失败段",
    )
    .unwrap();
    let list = skills::catalog(&root, "").unwrap();
    assert_eq!(
        list.as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == id)
            .unwrap()["available"],
        true
    );
    assert_eq!(
        list.as_array()
            .unwrap()
            .iter()
            .find(|s| s["id"] == "creative-ad-director")
            .unwrap()["available"],
        false
    );
    let page = skills::read(&root, "", id, "SKILL.md", 0, true).unwrap();
    assert_eq!(page["nextOffset"], 4000);
    assert!(
        page["resources"]
            .as_array()
            .unwrap()
            .contains(&json!("references/actions.md"))
    );
    let second = skills::read(&root, "", id, "SKILL.md", 4000, true).unwrap();
    assert_eq!(
        page["text"].as_str().unwrap().to_owned() + second["text"].as_str().unwrap(),
        rules
    );
    let cross = "../product-video-production/SKILL.md#section";
    assert_eq!(
        skills::read(&root, "", id, cross, 0, true).unwrap()["text"],
        "只修改失败段"
    );
    let enabled = "[\"skill-product-storyboard\"]";
    assert!(skills::read(&root, enabled, id, cross, 0, true).is_ok());
    assert!(
        skills::read(
            &root,
            enabled,
            "product-video-production",
            "SKILL.md",
            0,
            true
        )
        .is_err()
    );
    assert!(skills::read(&root, "[]", id, "SKILL.md", 0, true).is_err());
    assert!(skills::read(&root, "[]", id, "SKILL.md", 0, false).is_ok());
    fs::write(root.join("private.md"), "not a rule").unwrap();
    assert!(skills::read(&root, "", id, "../private.md", 0, true).is_err());
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(root.join("private.md"), root.join(id).join("escape.md"))
            .unwrap();
        assert!(skills::read(&root, "", id, "escape.md", 0, true).is_err());
    }
    let guide = skills::guidance(&skills::catalog(&root, "[]").unwrap());
    assert!(guide.contains("skills：[]"));
    fs::remove_dir_all(root).unwrap();
}
