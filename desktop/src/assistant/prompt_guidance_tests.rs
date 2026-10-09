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
    let profile = quick_profile(&db, "video").unwrap();
    assert_eq!(profile.id, "production");
    let context = json!({"models":{"video":"h3"}});
    let brief = "Shot 1, 0-3s: the lead lowers the bag; the coworker leans in and asks a question. Shot 2, 3-6s: one hand opens the zipper and lifts the lid.";
    let guidance = for_quick(&db, &profile, "video", brief, &context, &json!({})).unwrap();
    // The bound entry document and its core come from the database.
    assert!(guidance.contains("product-video-production/SKILL.md"));
    assert!(guidance.contains("product-video-production/CORE.md"));
    // The bound Skill owns its motion, performance and timing methods.
    assert!(guidance.contains("## Human performance"));
    assert!(guidance.contains("## Timing and selection"));
    // The prompt-writing guide is always present for the bound kind.
    assert!(guidance.contains("product-video-production/references/prompt-writing.md"));
    assert!(guidance.contains("### product-video-production/references/shot-execution.md"));
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
    assert!(short.contains("### product-video-production/references/shot-execution.md"));
    assert!(!short.contains("### creative-ad-director/"));
    db.execute("UPDATE skill_resources SET text=text || '\nCURRENT_SPATIAL_RULE' WHERE skill_id='product-video-production' AND path='references/shot-execution.md'", []).unwrap();
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
    assert!(portable.contains("product-video-production/references/prompt-writing.md"));
    assert!(!portable.contains("MiniMax H3 prompt adapter"));

    // The image panel binds the image Skill, not the video one.
    let image = quick_profile(&db, "image").unwrap();
    let image_guidance = for_quick(
        &db,
        &image,
        "image",
        "a storyboard key frame of the bag on a desk",
        &json!({"models":{"image":"image"}}),
        &json!({}),
    )
    .unwrap();
    assert!(image_guidance.contains("image-production/SKILL.md"));
    assert!(!image_guidance.contains("CURRENT_SPATIAL_RULE"));
    assert!(image_guidance.contains("image-production/references/scene-execution.md"));
    assert!(image_guidance.contains("### image-production/references/frames.md"));
    assert!(!image_guidance.contains("### product-video-production/"));
    assert!(!image_guidance.contains("### creative-ad-director/"));
    assert!(!image_guidance.contains("product-video-production/references/control.md"));
    drop(db);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

/// The real audited task (`job 1791390322635289000-7`, project 10-7-2): a 15-second
/// live-action brief whose submitted prompt packed timed multi-event action around
/// real people, while its output was never inspected. Whitespace is normalized for
/// the fixture; the task signals the router reads are unchanged.
#[test]
fn the_audited_real_brief_selects_the_methods_that_were_missing() {
    let (root, store) = fixture();
    let db = store.db.lock().unwrap();
    let profile = quick_profile(&db, "video").unwrap();
    let context = json!({"models":{"video":"h3"}});
    let real_brief = r#"Generate a 15-second 9:16 vertical photorealistic live-action TikTok advertisement for the US market. Natural contemporary office lunch area, soft daylight, believable adult coworkers, realistic skin and woven fabric. Keep people, wardrobe, desk, light and the exact same blue lunch bag consistent across clean hard cuts. The supplied product photo is authoritative for bag geometry, proportions, fabric and hardware; do not copy its white background or FRONT label. Shot 1, 0–3s: Medium desk-level shot. Lead lowers the closed blue lunch bag onto the desk; its base makes full contact before the hand releases it. Coworker leans in, looks at the bag and asks in natural American English: 'Where are we eating today?' Lead rests one hand near the top zipper and gives a small amused glance. Shot 2, 3–6s: Clear close three-quarter view. One hand steadies the bag while the other pulls a slider along the existing top zipper track. Only after unzipping, lift the lid on its rear fabric connection, revealing an appropriately sized closed lunch container fully inside. Lead: 'My place.' Grasp the container, lift vertically through the open top with clearance, and lower onto the clear desk beside the bag. Maintain hand support until its base lands. A motivated hard cut between opening and extraction may keep each action readable. Shot 3, 6–10s: Food close-up with the bag still visible. Open the container to reveal an appetizing colorful lunch. Unzip the bag's real front pocket and take out a slim wrapped cutlery set that fits entirely inside it. Draw it through the opened pocket mouth and set it beside the food. Keep fingers, pocket and utensils visibly separate. English on-screen text: 'Your favorites. Packed.' Shot 4, 10–12s: Two-person medium shot. Coworker glances at their plain lunch and then the lead's food, moves their chair slightly closer and asks: 'Got room for one more?' Lead smiles and places a hand protectively beside the container, without touching the food. Shot 5, 12–15s: Hard cut to the same bag closed, lid aligned and both top zipper pulls present. Lead grips the padded handle, takes up tension, then lifts a short distance; the intact bag hangs naturally below the hand. Cut to a steady front-facing product close-up of the bag resting on the desk. Show recognizable blue fabric, black handles, silver zipper pulls, front zip pocket and visible side fittings. Hold the ending for readable English text: 'Lunch, handled.' and 'Shop now.' Physical realism is essential: retain the rectangular bag shape and stable volume; preserve blue woven fabric, black perimeter zipper and two silver pulls, both black handle straps and blue padded grip, rounded rectangular front zip pocket, side mesh pockets, shoulder strap and side attachment clips. Allow small realistic fabric creases while preserving construction. Hands grip surfaces with anatomically plausible fingers; fingers never intersect fabric or hardware. Every container and utensil transfer passes through an actually opened aperture with adequate clearance. Show support, path, contact and release in that order. No object crosses closed walls, zippers or tabletop. No melting, stretching, shape morphing, duplicated or missing hardware, fused fingers, floating objects or teleporting transfers. Keep occluded components consistent when they reappear. Use clear stable angles, ordinary movement and clean cuts, without extreme camera orbits or speed ramps. Audio: low office room tone and light upbeat music beneath clear natural American-English dialogue. Synchronize bag landing, zipper sliding, lunch-box opening and utensil contact with visible actions. Lower music briefly under the coworker's final question and restore it for the product ending. Preserve the full 15-second story, dialogue and captions."#;
    let guidance = for_quick(&db, &profile, "video", real_brief, &context, &json!({})).unwrap();
    // The bound entry, its core and its own prompt guide come from the database.
    assert!(guidance.contains("product-video-production/SKILL.md"));
    assert!(guidance.contains("product-video-production/CORE.md"));
    assert!(guidance.contains("product-video-production/references/prompt-writing.md"));
    // Every method the audit found missing for this exact task is now selected:
    // real people and dialogue, timed multi-event action, photographic appearance
    // and physical contact.
    for expected in [
        "## Human performance",
        "## Timing and selection",
        "## Appearance and continuity",
        "## Contact and motion",
    ] {
        assert!(guidance.contains(expected), "missing route: {expected}");
    }
    // The rules that forbid writing a full operating sequence are present.
    assert!(guidance.contains("Physical causality is not a full operating sequence"));
    // Every input must be bound to a subject and visual use.
    assert!(guidance.contains("concrete purpose"));
    drop(db);
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn quick_preparation_fails_if_its_required_local_method_is_missing() {
    let (root, store) = fixture();
    let db = store.db.lock().unwrap();
    let profile = quick_profile(&db, "video").unwrap();
    db.execute("DELETE FROM skill_resources WHERE skill_id='product-video-production' AND path='references/shot-execution.md'", []).unwrap();
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
fn video_motion_method_uses_current_db_text_and_stays_the_same_across_models() {
    let (root, store) = fixture();
    let db = store.db.lock().unwrap();
    let profile = quick_profile(&db, "video").unwrap();
    let brief = "按附件脚本整理视频提示词";
    let generic = json!({"models":{"video":"generic-video"}});
    let original = for_quick(&db, &profile, "video", brief, &generic, &json!({})).unwrap();
    let page = crate::assistant::skills::storage::read(
        &db,
        "",
        "product-video-production",
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
        "product-video-production",
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
    let image = quick_profile(&db, "image").unwrap();
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
