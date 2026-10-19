use super::*;

#[test]
fn numeric_versions_match_across_domain_round_trips_without_losing_precision() {
    let value = |raw: &str| serde_json::from_str::<Value>(raw).unwrap();
    for (left, right) in [
        ("5.0", "5"),
        ("-5.0", "-5"),
        ("-0.0", "0"),
        ("0.0", "0"),
        ("9.223372036854776e18", "9223372036854775808"),
        ("-9.223372036854776e18", "-9223372036854775808"),
    ] {
        assert_eq!(version(&value(left)), version(&value(right)), "{left}");
    }
    for (left, right) in [
        ("5", "5.25"),
        ("9007199254740992", "9007199254740993"),
        ("18446744073709551615", "1.8446744073709552e19"),
        ("-9223372036854775808", "-9.223372036854778e18"),
    ] {
        assert_ne!(version(&value(left)), version(&value(right)), "{left}");
    }
    assert_eq!(
        version(&json!({"nested":[{"duration":5.0,"start":-0.0}]})),
        version(&json!({"nested":[{"start":0,"duration":5}]}))
    );
}

#[test]
fn observation_accepts_equivalent_numbers_but_rejects_real_changes() {
    let (store, _, _) = crate::project_service::tests::fixture();
    let db = store.db.lock().unwrap();
    let before = json!({"assets":[{"id":"video","duration":5.0}]});
    let mut after = json!({"assets":[{"id":"video","duration":5}]});
    let keys = BTreeSet::from(["asset:video".into()]);
    remember(&db, "p", "turn", &keys, &before).unwrap();
    assert!(check(&db, "p", "turn", &keys, &after).unwrap().is_none());
    assert!(required(&before, &after, &json!({})).is_empty());
    after["assets"][0]["duration"] = json!(5.25);
    let conflict = check(&db, "p", "turn", &keys, &after).unwrap().unwrap();
    assert_eq!(conflict["conflicts"][0]["code"], "TARGET_CHANGED");
    assert_eq!(required(&before, &after, &json!({})), keys);
    drop(db);
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn private_previs_does_not_block_unrelated_or_repeated_edits() {
    use crate::project_service::{execute, load};
    let (store, _, profile) = crate::project_service::tests::fixture();
    let asset = mstudio::model::Asset {
        attachment_only: false,
        missing: false,
        generated: true,
        id: "previs".into(),
        name: "Previs".into(),
        kind: "video".into(),
        path: store.root.join("previs.mp4").to_str().unwrap().into(),
        preview: String::new(),
        duration: 5.0,
        width: 720,
        height: 1280,
        has_audio: false,
    };
    std::fs::write(&asset.path, b"synthetic registered media").unwrap();
    crate::previs::media::register(&store.db.lock().unwrap(), "p", &asset).unwrap();
    execute(
        &store,
        &profile,
        "p",
        "turn",
        "read",
        json!({
            "action":"inspect", "section":"creation"
        }),
    )
    .unwrap();
    for index in 0..3 {
        let receipt = execute(
            &store,
            &profile,
            "p",
            "turn",
            &format!("edit-{index}"),
            json!({
                "action":"edit", "operations":[{"op":"set_brief","text":format!("Brief {index}")}]
            }),
        )
        .unwrap();
        assert_eq!(receipt["outcome"], "committed", "{receipt}");
    }
    let observed: i64 = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT count(*) FROM project_observations WHERE target='asset:previs'",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(
        observed, 0,
        "Unrelated edits must not observe or modify previs"
    );
    let read = execute(
        &store,
        &profile,
        "p",
        "turn",
        "read-previs",
        json!({
            "action":"inspect", "section":"assets", "ids":["previs"]
        }),
    )
    .unwrap();
    assert_eq!(read["items"][0]["id"], "previs", "{read}");
    for index in 0..3 {
        let receipt = execute(&store, &profile, "p", "turn", &format!("creation-{index}"), json!({
            "action":"edit", "operations":[{"op":"set_creation","preserve":format!("Keep {index}")}]
        })).unwrap();
        assert_eq!(receipt["outcome"], "committed", "{receipt}");
        let db = store.db.lock().unwrap();
        let doc = load(&db, "p").unwrap();
        assert!(
            check(
                &db,
                "p",
                "turn",
                &BTreeSet::from(["asset:previs".into()]),
                &doc
            )
            .unwrap()
            .is_none()
        );
    }
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
