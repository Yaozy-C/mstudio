use ges::prelude::*;
use gstreamer as gst;
use gstreamer_editing_services as ges;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
// GES position queries can jump to a composition boundary at non-unit rates.
// Use the PTS of the delivered video frame as the preview clock instead.
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
// A seek returning Ok only means that its event was accepted. Wait for
// preroll/state completion before acknowledging it to the command queue.
pub(super) fn settle(pipeline: &ges::Pipeline) -> Result<(), String> {
    let (result, _, _) = pipeline.state(gst::ClockTime::from_seconds(5));
    match result {
        Ok(gst::StateChangeSuccess::Async) => Err("预览定位等待超时，请重试".into()),
        Ok(_) => Ok(()),
        Err(error) => Err(format!("预览状态切换失败：{error}")),
    }
}
pub(super) fn seek(
    pipeline: &ges::Pipeline,
    frames: &Mutex<Vec<u8>>,
    frame: i32,
    fps: u32,
    resume: bool,
    rate: f64,
    duration: f64,
) -> Result<(), String> {
    let previous_sequence = frames
        .lock()
        .unwrap()
        .get(..4)
        .map(|v| u32::from_le_bytes(v.try_into().unwrap()));
    // Seek while paused so its preroll cannot run past the requested frame.
    pipeline
        .set_state(gst::State::Paused)
        .map_err(|e| e.to_string())?;
    pipeline
        .seek(
            rate,
            gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
            gst::SeekType::Set,
            time(frame as f64 / fps as f64),
            gst::SeekType::Set,
            time(duration),
        )
        .map_err(|e| e.to_string())?;
    settle(pipeline)?;
    // At EOS get_state can finish before the new preroll callback. The frame
    // packet is the authoritative completion signal used by the WebView.
    let deadline = Instant::now() + Duration::from_secs(5);
    loop {
        let ready = {
            let packet = frames.lock().unwrap();
            packet.len() > 16
                && Some(u32::from_le_bytes(packet[..4].try_into().unwrap())) != previous_sequence
                && u32::from_le_bytes(packet[12..16].try_into().unwrap()) == frame as u32
        };
        if ready {
            break;
        }
        if Instant::now() >= deadline {
            return Err("预览定位等待画面超时，请重新加载预览".into());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
    if resume {
        pipeline
            .set_state(gst::State::Playing)
            .map_err(|e| e.to_string())?;
        settle(pipeline)?;
    }
    Ok(())
}
