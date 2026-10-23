use super::*;
use serde_json::json;

#[test]
fn unknown_audio_video_duration_is_rejected_instead_of_becoming_five_seconds() {
    for kind in ["audio", "video"] {
        for value in [
            Value::Null,
            json!("N/A"),
            json!("NaN"),
            json!("0"),
            json!("-2"),
        ] {
            let metadata = json!({"format":{"duration":value},"streams":[{"duration":"12.5"}]});
            assert!(import_duration(kind, &metadata).is_err(), "{kind}: {value}");
        }
        assert_eq!(
            import_duration(kind, &json!({"format":{"duration":"12.5"}})).unwrap(),
            12.5
        );
    }
    assert_eq!(import_duration("image", &json!({})).unwrap(), 5.0);
}

#[test]
fn picture_dimensions_must_be_known_positive_and_representable() {
    for stream in [
        json!({}),
        json!({"width":0,"height":720}),
        json!({"width":1280}),
        json!({"width":1280,"height":-1}),
        json!({"width":u64::MAX,"height":720}),
    ] {
        assert!(picture_dimensions(&stream).is_err(), "{stream}");
    }
    assert_eq!(
        picture_dimensions(&json!({"width":1280,"height":720})).unwrap(),
        (1280, 720)
    );
}
