use super::time;
use ges::prelude::*;
use gstreamer as gst;
use gstreamer_editing_services as ges;
use mstudio::preview_ges::Plan;
use std::sync::{Arc, Mutex};
pub(super) fn build(plan: &Plan, frames: Arc<Mutex<Vec<u8>>>) -> anyhow::Result<ges::Pipeline> {
    gst::init()?;
    ges::init()?;
    // VideoToolbox can return an opaque black frame when GES seeks across
    // overlapping H.264 layers. Use the bundled software decoder for this
    // bounded-resolution preview, including normal playback through a seam.
    #[cfg(target_os = "macos")]
    for name in ["vtdec", "vtdec_hw"] {
        if let Some(factory) = gst::ElementFactory::find(name) {
            factory.set_rank(gst::Rank::NONE);
        }
    }
    for name in [
        "nlecomposition",
        "compositor",
        "videoflip",
        "imagefreeze",
        "encodebin2",
        "appsink",
        "audiomixer",
        "volume",
    ] {
        anyhow::ensure!(
            gst::ElementFactory::find(name).is_some(),
            "GES 运行时缺少插件：{name}"
        );
    }
    let timeline = ges::Timeline::new_audio_video();
    for track in timeline.tracks() {
        if track.track_type() == ges::TrackType::VIDEO {
            track.set_restriction_caps(
                &gst::Caps::builder("video/x-raw")
                    .field("width", plan.width as i32)
                    .field("height", plan.height as i32)
                    .field("framerate", gst::Fraction::new(plan.fps as i32, 1))
                    .field("pixel-aspect-ratio", gst::Fraction::new(1, 1))
                    .build(),
            );
        }
    }
    // GES priority 0 is topmost; saved project order is bottom to top.
    for layer in plan.layers.iter().rev() {
        let clip = ges::UriClip::new(ges::gio::File::for_path(&layer.path).uri().as_str())?;
        clip.set_supported_formats(ges::TrackType::VIDEO);
        anyhow::ensure!(
            clip.set_start(time(layer.start))
                && clip.set_inpoint(time(layer.trim))
                && clip.set_duration(time(layer.duration)),
            "GES 片段时间无效"
        );
        timeline.append_layer().add_clip(&clip)?;
        for (name, value) in [
            ("posx", layer.x),
            ("posy", layer.y),
            ("width", layer.width),
            ("height", layer.height),
        ] {
            clip.set_child_property(name, value)?;
        }
        clip.set_child_property("alpha", layer.opacity)?;
    }
    if let Some(path) = &plan.audio {
        let clip = ges::UriClip::new(ges::gio::File::for_path(path).uri().as_str())?;
        clip.set_supported_formats(ges::TrackType::AUDIO);
        clip.set_duration(time(plan.duration));
        timeline.append_layer().add_clip(&clip)?;
    }
    let black =
        ges::TestClip::for_nick("black").ok_or_else(|| anyhow::anyhow!("GES 黑场不可用"))?;
    black.set_supported_formats(ges::TrackType::VIDEO);
    black.set_duration(time(plan.duration));
    timeline.append_layer().add_clip(&black)?;
    anyhow::ensure!(timeline.commit(), "GES 时间线提交失败");
    let pipeline = ges::Pipeline::new();
    pipeline.set_timeline(&timeline)?;
    let sink = gstreamer_app::AppSink::builder()
        .caps(
            &gst::Caps::builder("video/x-raw")
                .field("format", "RGBA")
                .build(),
        )
        .sync(true)
        .max_buffers(1)
        .drop(true)
        .build();
    let count = Arc::new(std::sync::atomic::AtomicU32::new(0));
    let fps = plan.fps;
    let a = frames.clone();
    let count2 = count.clone();
    sink.set_callbacks(
        gstreamer_app::AppSinkCallbacks::builder()
            .new_sample(move |s| {
                let sample = s.pull_sample().map_err(|_| gst::FlowError::Eos)?;
                copy_frame(&sample, &a, &count, fps)
            })
            .new_preroll(move |s| {
                let sample = s.pull_preroll().map_err(|_| gst::FlowError::Eos)?;
                copy_frame(&sample, &frames, &count2, fps)
            })
            .build(),
    );
    pipeline.preview_set_video_sink(Some(&sink));
    let audio = gst::ElementFactory::make(if cfg!(test) {
        "fakesink"
    } else {
        "autoaudiosink"
    })
    .build()?;
    pipeline.preview_set_audio_sink(Some(&audio));
    if let Err(e) = pipeline.set_state(gst::State::Paused) {
        let _ = pipeline.set_state(gst::State::Null);
        return Err(e.into());
    }
    let (result, state, _) = pipeline.state(gst::ClockTime::from_seconds(15));
    if result.is_err() || state != gst::State::Paused {
        let message = pipeline
            .bus()
            .and_then(|b| b.pop_filtered(&[gst::MessageType::Error]))
            .and_then(|m| {
                if let gst::MessageView::Error(e) = m.view() {
                    Some(e.error().to_string())
                } else {
                    None
                }
            });
        let _ = pipeline.set_state(gst::State::Null);
        anyhow::bail!(
            "GES 首帧准备失败：{}",
            message.unwrap_or_else(|| "等待超时".into())
        );
    }
    Ok(pipeline)
}
fn copy_frame(
    sample: &gst::Sample,
    frames: &Mutex<Vec<u8>>,
    count: &std::sync::atomic::AtomicU32,
    fps: u32,
) -> Result<gst::FlowSuccess, gst::FlowError> {
    let info = gstreamer_video::VideoInfo::from_caps(sample.caps().ok_or(gst::FlowError::Error)?)
        .map_err(|_| gst::FlowError::Error)?;
    let buffer = sample.buffer().ok_or(gst::FlowError::Error)?;
    let frame = gstreamer_video::VideoFrameRef::from_buffer_ref_readable(buffer, &info)
        .map_err(|_| gst::FlowError::Error)?;
    let data = frame.plane_data(0).map_err(|_| gst::FlowError::Error)?;
    let position =
        (buffer.pts().unwrap_or(gst::ClockTime::ZERO).seconds_f64() * fps as f64).round() as u32;
    let seq = count
        .fetch_add(1, std::sync::atomic::Ordering::Relaxed)
        .wrapping_add(1);
    let mut packet = frames.lock().unwrap();
    packet.clear();
    for x in [seq, info.width(), info.height(), position] {
        packet.extend_from_slice(&x.to_le_bytes());
    }
    let width = info.width() as usize * 4;
    for row in 0..info.height() as usize {
        let start = row * info.stride()[0] as usize;
        packet.extend_from_slice(&data[start..start + width]);
    }
    Ok(gst::FlowSuccess::Ok)
}
