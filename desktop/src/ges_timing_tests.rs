use super::{Player, wait_frame};
use crate::ges_output::ObservedFrame;
use mstudio::{
    model::{Clip, RenderSpec, Track},
    preview_ges,
};
use std::{
    path::Path,
    time::{Duration, Instant},
};

fn wait_rate(received: &std::sync::mpsc::Receiver<ObservedFrame>, rate: f64, after: u32) -> u32 {
    let deadline = Instant::now() + Duration::from_secs(4);
    loop {
        let frame = received
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .unwrap_or_else(|_| {
                panic!("native playback did not advance after {after} at rate {rate}")
            });
        assert!(frame.red > 40, "black playback frame {}", frame.position);
        if frame.position > after && (frame.rate - rate).abs() < 1e-6 {
            return frame.position;
        }
    }
}

pub(super) fn playback_rates(p: &Player, received: &std::sync::mpsc::Receiver<ObservedFrame>) {
    // Verify the rate on actual GStreamer samples, independent of runner scheduling.
    for step in [8, 1] {
        let rate = f64::from(step) / 4.;
        p.control("rate", step).unwrap();
        let sought = p.control("seek", 0).unwrap();
        assert_eq!(sought.rate, rate);
        received.try_iter().for_each(drop);
        p.control("play", 0).unwrap();
        let frame = wait_rate(received, rate, 0);
        let paused = p.control("pause", 0).unwrap();
        assert!(paused.frame >= frame as i32);
        assert_eq!(paused.rate, rate);
    }
    p.control("seek", 0).unwrap();
    received.try_iter().for_each(drop);
    p.control("play", 0).unwrap();
    let changed = p.control("rate", 6).unwrap();
    assert!(changed.playing);
    assert_eq!(changed.rate, 1.5);
    let frame = wait_rate(received, 1.5, changed.frame as u32);
    let paused = p.control("pause", 0).unwrap();
    assert!(paused.frame >= frame as i32);
    assert_eq!(paused.rate, 1.5);
    p.control("rate", 4).unwrap();
}

/// Reuse the existing fixture; validate real cache preparation and GES together.
pub(super) fn cached_fractional_cuts(root: &Path, source: &Path) {
    use gstreamer_editing_services::{self as ges, prelude::*};
    let timeline = ges::Timeline::new_audio_video();
    for fps in [24, 25, 30, 60] {
        for track in timeline.tracks() {
            if track.track_type() == ges::TrackType::VIDEO {
                track.set_restriction_caps(
                    &gstreamer::Caps::builder("video/x-raw")
                        .field("framerate", gstreamer::Fraction::new(fps as i32, 1))
                        .build(),
                );
            }
        }
        for frame in [0, 1, 5, 124, 248] {
            let ns = mstudio::preview_clock::frame_ns(frame as u64, fps);
            assert_eq!(timeline.frame_time(frame).unwrap().nseconds(), ns);
            assert_eq!(
                timeline.frame_at(gstreamer::ClockTime::from_nseconds(ns)),
                frame
            );
        }
    }
    let asset = mstudio::media::import(source, root).unwrap();
    let mut spec = RenderSpec {
        width: 160,
        height: 90,
        fps: 30,
        tracks: vec![Track {
            id: "v".into(),
            kind: "video".into(),
            hidden: false,
            muted: true,
        }],
        clips: vec![],
        captions: vec![],
    };
    for duration in [0.1472, 5. / 30.] {
        spec.clips = (0..3)
            .map(|i| Clip {
                id: format!("cut-{i}"),
                asset_id: asset.id.clone(),
                track_id: "v".into(),
                trim_in: 0.,
                trim_out: duration,
                start: i as f64 * duration,
                speed: 1.,
                volume: 1.,
                visual: Some(serde_json::from_str(r#"{"effect":"grayscale"}"#).unwrap()),
                ..Default::default()
            })
            .collect();
        let plan =
            preview_ges::prepare(&spec, std::slice::from_ref(&asset), &root.join("cache"), 0)
                .unwrap();
        assert_eq!(plan.duration, 3. * duration);
        for (clip, layer) in spec.clips.iter().zip(&plan.layers) {
            assert_eq!(clip.start, layer.start);
            assert_eq!(clip.duration(), layer.duration);
        }
        let (sent, received) = std::sync::mpsc::channel();
        let p =
            Player::open_with_output(plan, crate::ges_output::FrameOutput::Observed(sent)).unwrap();
        let total = p.snapshot().total as u32;
        for target in (0..total).rev().chain(0..total) {
            p.control("seek", target as i32).unwrap();
            let f = wait_frame(&p, target);
            assert!(
                f[16] > 40,
                "cache/seek black at {target}, duration {duration}"
            );
        }
        p.control("seek", 0).unwrap();
        // Discard seek/preroll samples; observe every playback callback, rather
        // than assuming a polling consumer sees each overwritten latest frame.
        received.try_iter().for_each(drop);
        p.control("play", 0).unwrap();
        let deadline = Instant::now() + Duration::from_secs(4);
        loop {
            let status = p.snapshot();
            assert!(
                status.error.is_none(),
                "playback failed: {:?}",
                status.error
            );
            if status.phase == "ended" {
                break;
            }
            assert!(Instant::now() < deadline, "fractional playback timed out");
            std::thread::sleep(Duration::from_millis(10));
        }
        let mut seen = std::collections::BTreeSet::new();
        for frame in received.try_iter() {
            assert!(
                frame.red > 40,
                "cache/play black at {}, duration {duration}",
                frame.position
            );
            seen.insert(frame.position);
        }
        for cut in [duration, 2. * duration] {
            let boundary = (cut * 30.).ceil() as u32;
            assert!(
                seen.contains(&boundary),
                "cut frame {boundary} was not observed"
            );
        }
        assert!(seen.contains(&(total - 1)), "final frame was not observed");
    }
}
