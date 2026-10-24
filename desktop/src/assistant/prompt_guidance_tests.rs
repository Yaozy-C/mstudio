use super::*;
use crate::database::Store;

fn fixture() -> (std::path::PathBuf, Store) {
    let root = std::env::temp_dir().join(format!("mstudio-prompt-rules-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    {
        let db = store.db.lock().unwrap();
        crate::assistant::skills::storage::seed(
            &db,
            &std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills"),
        )
        .unwrap();
        let models = json!([
            {"id":"h3","name":"Selected video","plugin":"fal","kind":"video","endpoint":"minimax/h3/reference-to-video","enabled":true,"params":{}},
            {"id":"generic-video","name":"Other video","plugin":"fal","kind":"video","endpoint":"example/video","enabled":true,"params":{}},
            {"id":"image","name":"Image","plugin":"codex-image","kind":"image","endpoint":"codex://local/images","enabled":true,"params":{}}
        ]);
        db.execute(
            "INSERT INTO settings VALUES('media-models',?1)",
            [models.to_string()],
        )
        .unwrap();
    }
    (root, store)
}

#[test]
fn model_guidance_is_selected_without_eagerly_loading_skill_bodies() {
    let (root, store) = fixture();
    let db = store.db.lock().unwrap();
    let animation = format!("{}\nCURRENT_ANIMATION_TAIL", "x".repeat(9000));
    db.execute(
        "UPDATE skill_resources SET text=?1 WHERE path='references/animation-principles.md'",
        [&animation],
    )
    .unwrap();
    let context = json!({"models":{"video":"h3","image":"image"}});
    for id in [
        "coordinator",
        "concept",
        "director",
        "image",
        "production",
        "editor",
    ] {
        let profile = crate::assistant::profiles::builtins()
            .into_iter()
            .find(|a| a.id == id)
            .unwrap();
        let result = for_agent(&db, &profile, &context, &json!({})).unwrap();
        assert!(!result.contains("CURRENT_ANIMATION_TAIL"), "{id}");
        assert!(!result.contains("Assigned Skill rule"), "{id}");
    }
    let production = crate::assistant::profiles::builtins()
        .into_iter()
        .find(|a| a.id == "production")
        .unwrap();
    assert!(
        for_agent(&db, &production, &context, &json!({}))
            .unwrap()
            .contains("MiniMax H3 prompt adapter")
    );
    drop(db);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn quick_prompt_preparation_receives_the_current_database_bodies() {
    let (root, store) = fixture();
    let db = store.db.lock().unwrap();
    let profile = quick_profile(&db, "video", None).unwrap();
    assert_eq!(profile.id, "production");
    let context = json!({"models":{"video":"h3"}});
    let brief = "Shot 1, 0-3s: the lead lowers the bag; the coworker leans in and asks a question. Shot 2, 3-6s: one hand opens the zipper and lifts the lid.";
    let guidance = for_quick(&db, &profile, "video", brief, &context, &json!({})).unwrap();
    // The bound entry document and its core come from the database.
    assert!(guidance.contains("storyboard-video-production/SKILL.md"));
    assert!(guidance.contains("storyboard-video-production/CORE.md"));
    // The bound Skill owns its motion, performance and timing methods.
    for expected in [
        "## Human performance",
        "## Timing and selection",
        "## Appearance and continuity",
        "## Contact and motion",
        "including necessary intermediate contact, support, grip and release states",
        "a continuous shot cannot skip a necessary transition",
        "concrete purpose",
    ] {
        assert!(
            guidance.contains(expected),
            "missing method rule: {expected}"
        );
    }
    // The prompt-writing guide is always present for the bound kind.
    assert!(guidance.contains("storyboard-video-production/references/prompt-writing.md"));
    assert!(guidance.contains("### storyboard-video-production/references/shot-execution.md"));
    assert!(guidance.contains("### storyboard-video-production/references/motion-realism.md"));
    // Even a vague request needs local spatial rules, but never another role's library.
    let short = for_quick(
        &db,
        &profile,
        "video",
        "做一条包包的视频",
        &context,
        &json!({}),
    )
    .unwrap();
    assert!(short.contains("### storyboard-video-production/references/shot-execution.md"));
    assert!(!short.contains("### creative-ad-director/"));
    db.execute("UPDATE skill_resources SET text=text || '\nCURRENT_SPATIAL_RULE' WHERE skill_id='storyboard-video-production' AND path='references/shot-execution.md'", []).unwrap();
    let attached = for_quick(
        &db,
        &profile,
        "video",
        "按附件脚本整理",
        &context,
        &json!({}),
    )
    .unwrap();
    assert!(attached.contains("CURRENT_SPATIAL_RULE"));
    let portable = for_quick(
        &db,
        &profile,
        "video",
        "按附件脚本整理",
        &json!({"models":{"video":"generic-video"}}),
        &json!({}),
    )
    .unwrap();
    // A model switch changes adaptation, while the current shot method still loads.
    assert!(portable.contains("CURRENT_SPATIAL_RULE"));
    assert!(portable.contains("storyboard-video-production/references/prompt-writing.md"));
    assert!(!portable.contains("MiniMax H3 prompt adapter"));

    // The image panel binds the image Skill, not the video one.
    let image = quick_profile(&db, "image", None).unwrap();
    let image_guidance = for_quick(
        &db,
        &image,
        "image",
        "a storyboard key frame of the bag on a desk",
        &json!({"models":{"image":"image"}}),
        &json!({}),
    )
    .unwrap();
    assert!(image_guidance.contains("storyboard-image-production/SKILL.md"));
    assert!(!image_guidance.contains("CURRENT_SPATIAL_RULE"));
    assert!(image_guidance.contains("storyboard-image-production/references/scene-execution.md"));
    assert!(image_guidance.contains("### storyboard-image-production/references/frames.md"));
    assert!(
        image_guidance
            .contains("### storyboard-image-production/references/photographic-direction.md")
    );
    assert!(!image_guidance.contains("### storyboard-image-production/references/skin-realism.md"));
    let portrait = for_quick(
        &db,
        &image,
        "image",
        "a portrait of a woman by a window",
        &json!({"models":{"image":"image"}}),
        &json!({}),
    )
    .unwrap();
    assert!(portrait.contains("### storyboard-image-production/references/skin-realism.md"));
    assert!(!image_guidance.contains("### storyboard-video-production/"));
    assert!(!image_guidance.contains("### creative-ad-director/"));
    assert!(!image_guidance.contains("storyboard-video-production/references/control.md"));
    drop(db);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn quick_preparation_fails_if_its_required_local_method_is_missing() {
    let (root, store) = fixture();
    let db = store.db.lock().unwrap();
    let profile = quick_profile(&db, "video", None).unwrap();
    db.execute("DELETE FROM skill_resources WHERE skill_id='storyboard-video-production' AND path='references/shot-execution.md'", []).unwrap();
    assert!(
        for_quick(
            &db,
            &profile,
            "video",
            "use attached brief",
            &json!({}),
            &json!({})
        )
        .is_err()
    );
    drop(db);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn asset_prompt_preparation_uses_asset_rules_even_when_the_description_mentions_storyboards() {
    let (root, store) = fixture();
    let db = store.db.lock().unwrap();
    let profile = quick_profile(&db, "image", Some("asset")).unwrap();
    assert_eq!(profile.id, "assets");
    let guidance = for_quick(
        &db,
        &profile,
        "image",
        "为后续分镜准备人物资产",
        &json!({"models":{"image":"image"}}),
        &json!({}),
    )
    .unwrap();
    assert!(guidance.contains("### image-production/SKILL.md"));
    assert!(guidance.contains("### image-production/references/reference-assets.md"));
    assert!(!guidance.contains("storyboard-image-production/"));
    assert!(!guidance.contains("references/frames.md"));
    assert!(quick_profile(&db, "video", Some("asset")).is_err());
    assert!(quick_profile(&db, "image", Some("unknown")).is_err());
    drop(db);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn video_motion_method_uses_current_db_text_and_stays_the_same_across_models() {
    let (root, store) = fixture();
    let db = store.db.lock().unwrap();
    let profile = quick_profile(&db, "video", None).unwrap();
    let brief = "按附件脚本整理视频提示词";
    let generic = json!({"models":{"video":"generic-video"}});
    let original = for_quick(&db, &profile, "video", brief, &generic, &json!({})).unwrap();
    let page = crate::assistant::skills::storage::read(
        &db,
        "",
        "storyboard-video-production",
        "references/prompt-writing.md",
        0,
        true,
    )
    .unwrap();
    let custom = format!(
        "{}\nDB_SPECIFIC_MOTION_RULE",
        page["text"].as_str().unwrap()
    );
    crate::assistant::skills::storage::save(
        &db,
        "storyboard-video-production",
        "references/prompt-writing.md",
        &custom,
        page["revision"].as_i64().unwrap(),
    )
    .unwrap();
    let updated = for_quick(&db, &profile, "video", brief, &generic, &json!({})).unwrap();
    let h3 = for_quick(
        &db,
        &profile,
        "video",
        brief,
        &json!({"models":{"video":"h3"}}),
        &json!({}),
    )
    .unwrap();
    assert!(!original.contains("DB_SPECIFIC_MOTION_RULE"));
    assert!(updated.contains("DB_SPECIFIC_MOTION_RULE"));
    assert!(h3.contains("DB_SPECIFIC_MOTION_RULE"));
    // Model adaptation changes, while the current scene/motion method is identical.
    assert_eq!(
        updated.split("\nSelected model guidance (").next(),
        h3.split("\nSelected model guidance (").next()
    );
    assert!(!updated.contains("MiniMax H3 prompt adapter"));
    assert!(h3.contains("MiniMax H3 prompt adapter"));
    let image = quick_profile(&db, "image", None).unwrap();
    let still = for_quick(
        &db,
        &image,
        "image",
        brief,
        &json!({"models":{"image":"image"}}),
        &json!({}),
    )
    .unwrap();
    assert!(!still.contains("DB_SPECIFIC_MOTION_RULE"));
    drop(db);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
