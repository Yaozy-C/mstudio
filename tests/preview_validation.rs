use mstudio::{
    model::{Asset, Clip, RenderSpec, Track},
    preview_validate::validate,
};
fn fixture() -> (RenderSpec, Vec<Asset>) {
    let spec = RenderSpec {
        width: 1080,
        height: 1920,
        fps: 30,
        tracks: vec![Track {
            id: "v".into(),
            kind: "video".into(),
            hidden: false,
            muted: false,
        }],
        clips: vec![Clip {
            id: "c".into(),
            asset_id: "a".into(),
            track_id: "v".into(),
            trim_in: 2.,
            trim_out: 6.,
            speed: 2.,
            volume: 1.,
            start: 1.,
            ..Default::default()
        }],
        captions: vec![],
    };
    let assets = vec![Asset {
        id: "a".into(),
        name: "test".into(),
        kind: "video".into(),
        path: "/tmp/test.mp4".into(),
        preview: String::new(),
        duration: 10.,
        width: 480,
        height: 832,
        has_audio: true,
        missing: false,
        generated: false,
    }];
    (spec, assets)
}
#[test]
fn preview_validation_rejects_invalid_media_tracks_and_dimensions() {
    let (s, a) = fixture();
    assert!(validate(&s, &a, 640).is_ok());
    assert!(validate(&s, &a, 1280).is_ok());
    assert!(validate(&s, &a, 9000).is_err());
    assert!(validate(&s, &[], 640).is_err());
    let mut missing = a.clone();
    missing[0].missing = true;
    assert!(validate(&s, &missing, 640).is_err());
    let mut bad = s.clone();
    bad.clips[0].speed = f64::NAN;
    assert!(validate(&bad, &a, 640).is_err());
    let mut bad = s.clone();
    bad.tracks.push(bad.tracks[0].clone());
    assert!(validate(&bad, &a, 640).is_err());
    let mut bad = s.clone();
    bad.clips[0].track_id = "absent".into();
    assert!(validate(&bad, &a, 640).is_err());
    let mut bad = s.clone();
    bad.width = 0;
    assert!(validate(&bad, &a, 640).is_err());
    let mut bad = s.clone();
    bad.clips.clear();
    assert!(validate(&bad, &a, 640).is_err());
}
