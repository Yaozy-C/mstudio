use super::*;
use crate::model::{Clip, Track};
fn fixture() -> (RenderSpec, Vec<Asset>) {
    (
        RenderSpec {
            width: 1080,
            height: 1920,
            fps: 30,
            tracks: vec![Track {
                id: "v1".into(),
                kind: "video".into(),
                muted: false,
                hidden: false,
            }],
            clips: vec![Clip {
                id: "c".into(),
                asset_id: "a".into(),
                trim_in: 2.,
                trim_out: 6.,
                speed: 2.,
                volume: 0.5,
                start: 1.,
                track_id: "v1".into(),
                ..Default::default()
            }],
            captions: vec![],
        },
        vec![Asset {
            id: "a".into(),
            name: "test".into(),
            kind: "video".into(),
            path: "/tmp/a&b\".mp4".into(),
            preview: String::new(),
            duration: 10.,
            width: 480,
            height: 832,
            has_audio: true,
            missing: false,
            generated: false,
        }],
    )
}
#[test]
fn cuts_speed_and_gaps_use_project_frames() {
    let (s, a) = fixture();
    let text = graph(&s, &a).unwrap();
    assert!(text.contains("<blank length=\"30\"/>"));
    assert!(text.contains("<entry producer=\"p1\" in=\"30\" out=\"89\"/>"));
    assert!(text.contains("2:/tmp/a&amp;b&quot;.mp4"));
    assert!(text.contains("id=\"timeline\" in=\"0\" out=\"89\""));
    assert!(text.contains("name=\"sum\">1"));
}
#[test]
fn hidden_video_retains_audio_but_muted_video_remains_visible() {
    let (mut s, a) = fixture();
    s.tracks[0].hidden = true;
    let text = graph(&s, &a).unwrap();
    assert!(text.contains("hide=\"video\""));
    assert!(!text.contains("name=\"mlt_service\">affine"));
    assert!(text.contains("name=\"mlt_service\">mix"));
    s.tracks[0].hidden = false;
    s.tracks[0].muted = true;
    let text = graph(&s, &a).unwrap();
    assert!(text.contains("hide=\"audio\""));
    assert!(text.contains("name=\"mlt_service\">affine"));
    assert!(!text.contains("name=\"mlt_service\">mix"));
}
#[test]
fn zero_gain_does_not_override_other_audio_tracks() {
    let (mut s, a) = fixture();
    s.clips[0].volume = 0.;
    let text = graph(&s, &a).unwrap();
    assert!(text.contains("hide=\"audio\""));
    assert!(text.contains("name=\"mlt_service\">affine"));
    s.tracks[0].kind = "audio".into();
    let text = graph(&s, &a).unwrap();
    assert!(text.contains("hide=\"both\""));
    assert!(text.contains("name=\"audio_index\">-1"));
    assert!(!text.contains("name=\"mlt_service\">mix"));
}
#[test]
fn invalid_or_missing_media_fails_explicitly() {
    let (mut s, mut a) = fixture();
    a[0].missing = true;
    assert!(graph(&s, &a).is_err());
    a[0].missing = false;
    s.clips[0].speed = f64::NAN;
    assert!(graph(&s, &a).is_err());
}
