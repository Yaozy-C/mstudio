use super::asynchronous::{Events, Failure};
use ges::prelude::*;
use gstreamer as gst;
use gstreamer_editing_services as ges;
use std::{sync::Mutex, time::Instant};

// The delivered frame, rather than a possibly advanced composition query,
// is the authoritative preview position at every playback rate.
pub(super) fn displayed_frame(frames: &Mutex<Vec<u8>>) -> i32 {
    let packet = frames.lock().unwrap();
    packet
        .get(12..16)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()) as i32)
        .unwrap_or(0)
}
pub(super) fn time(s: f64) -> gst::ClockTime {
    gst::ClockTime::from_nseconds((s * 1e9).round() as u64)
}
pub(super) fn seek(
    pipeline: &ges::Pipeline,
    events: &mut Events,
    frames: &Mutex<Vec<u8>>,
    frame: i32,
    playing: bool,
    rate: f64,
    deadline: Instant,
) -> Result<(), Failure> {
    events.poll(pipeline)?;
    let before = events.done;
    let previous_sequence = frames
        .lock()
        .unwrap()
        .get(..4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()));
    let target = pipeline
        .timeline()
        .and_then(|timeline| timeline.frame_time(i64::from(frame)))
        .ok_or_else(|| Failure::Pipeline("GES 无法计算预览帧时间".into()))?;
    events.eos = false;
    // Match ges-launch: preserve playback state, seek accurately, and clear
    // an explicit stop instead of retaining the previous bounded segment.
    pipeline
        .seek(
            rate,
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            gst::SeekType::Set,
            target,
            gst::SeekType::Set,
            gst::ClockTime::NONE,
        )
        .map_err(|e| Failure::Pipeline(e.to_string()))?;
    events.wait(pipeline, Some(before), deadline, || {
        let packet = frames.lock().unwrap();
        if packet.len() < 16
            || Some(u32::from_le_bytes(packet[..4].try_into().unwrap())) == previous_sequence
        {
            return false;
        }
        let position = u32::from_le_bytes(packet[12..16].try_into().unwrap()) as i32;
        // Playing can advance after the target renders; paused seeks must
        // acknowledge the exact requested output frame.
        if playing {
            position >= frame
        } else {
            position == frame
        }
    })
}
