//! Frame timestamps match GES Timeline.get_frame_time (integer scaling with ceil).
//! Arbitrary edit times stay in seconds; only nanosecond-sized frame noise is removed.
const SECOND: u64 = 1_000_000_000;

pub fn frame_ns(frame: u64, fps: u32) -> u64 {
    (frame as u128 * SECOND as u128).div_ceil(fps as u128) as u64
}

pub fn timeline_ns(seconds: f64, fps: u32) -> u64 {
    let ns = (seconds * SECOND as f64).round() as u64;
    let frame = (seconds * fps as f64).round() as u64;
    let frame_ns = frame_ns(frame, fps);
    if ns.abs_diff(frame_ns) <= 1 {
        frame_ns
    } else {
        ns
    }
}

/// Count every output frame intersecting the requested duration, including the tail.
pub fn frame_count(seconds: f64, fps: u32) -> u64 {
    let ns = timeline_ns(seconds, fps);
    if ns == 0 {
        0
    } else {
        ((ns - 1) as u128 * fps as u128 / SECOND as u128) as u64 + 1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn frame_boundaries_match_ges_without_counting_an_extra_tail_frame() {
        for fps in [24, 25, 30, 60] {
            for frame in [0, 1, 124, 248, 373, 10_001] {
                let seconds = frame as f64 / fps as f64;
                assert_eq!(timeline_ns(seconds, fps), frame_ns(frame, fps));
                assert_eq!(frame_count(seconds, fps), frame);
            }
        }
        assert_eq!(frame_ns(248, 30), 8_266_666_667);
    }

    #[test]
    fn subframe_edits_are_preserved_and_cache_covers_the_tail() {
        assert_eq!(timeline_ns(4.1472, 30), 4_147_200_000);
        assert_eq!(frame_count(4.1472, 30), 125);
        assert_eq!(frame_count(0.000_001, 30), 1);
        let start = timeline_ns(124. / 30., 30);
        let end = timeline_ns(248. / 30., 30);
        assert_eq!(start + (end - start), 8_266_666_667);
    }
}
