use super::{
    Status,
    asynchronous::{Events, Failure},
    pipeline, seek, total_frames,
};
use crate::ges_output::FrameOutput;
use ges::prelude::*;
use gstreamer as gst;
use gstreamer_editing_services as ges;
use mstudio::preview_ges::Plan;
use std::{
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};

pub(super) struct Transport {
    pipeline: ges::Pipeline,
    events: Events,
    frames: Arc<Mutex<Vec<u8>>>,
    total: i32,
    playing: bool,
    ended: bool,
    rate: f64,
    error: Option<String>,
}
impl Transport {
    pub fn open(
        plan: &Plan,
        frames: Arc<Mutex<Vec<u8>>>,
        output: FrameOutput,
    ) -> Result<Self, String> {
        let pipeline = pipeline::build(plan, frames.clone(), output).map_err(|e| e.to_string())?;
        let mut transport = Self {
            pipeline,
            events: Events::default(),
            frames,
            total: total_frames(plan),
            playing: false,
            ended: false,
            rate: 1.,
            error: None,
        };
        transport
            .events
            .set_state(
                &transport.pipeline,
                gst::State::Paused,
                Instant::now() + Duration::from_secs(15),
            )
            .map_err(|e| e.to_string())?;
        Ok(transport)
    }
    pub fn snapshot(&self) -> Status {
        Status {
            frame: if self.ended {
                self.total - 1
            } else {
                seek::displayed_frame(&self.frames).clamp(0, self.total - 1)
            },
            playing: self.playing,
            total: self.total,
            rate: self.rate,
            phase: if self.error.is_some() {
                "error"
            } else if self.ended {
                "ended"
            } else if self.playing {
                "playing"
            } else {
                "paused"
            }
            .into(),
            error: self.error.clone(),
        }
    }
    pub fn poll(&mut self) -> Status {
        if self.error.is_none() {
            if let Err(error) = self.events.poll(&self.pipeline) {
                self.fail(error.to_string());
            } else if self.events.eos
                || (self.playing && seek::displayed_frame(&self.frames) >= self.total - 1)
            {
                self.ended = true;
                self.playing = false;
                self.events.eos = false;
                if let Err(error) = self.pipeline.set_state(gst::State::Paused) {
                    self.fail(error.to_string());
                }
            }
        }
        self.snapshot()
    }
    fn fail(&mut self, message: String) {
        self.error = Some(message);
        self.playing = false;
        let _ = self.pipeline.set_state(gst::State::Paused);
    }
    fn seek_to(&mut self, frame: i32, rate: f64, deadline: Instant) -> Result<(), Failure> {
        let first_deadline = deadline.min(Instant::now() + Duration::from_secs(3));
        match seek::seek(
            &self.pipeline,
            &mut self.events,
            &self.frames,
            frame,
            self.playing,
            rate,
            first_deadline,
        ) {
            Err(Failure::Timeout) => {
                // One bounded recovery, following Pitivi's READY -> PAUSED
                // rebuild. Retain the requested frame, speed and playback intent.
                self.events
                    .set_state(&self.pipeline, gst::State::Ready, deadline)?;
                self.events = Events::default();
                self.events
                    .set_state(&self.pipeline, gst::State::Paused, deadline)?;
                seek::seek(
                    &self.pipeline,
                    &mut self.events,
                    &self.frames,
                    frame,
                    false,
                    rate,
                    deadline,
                )?;
                if self.playing {
                    self.events
                        .set_state(&self.pipeline, gst::State::Playing, deadline)?;
                }
                Ok(())
            }
            result => result,
        }
    }
    pub fn control(&mut self, action: &str, frame: i32) -> Result<Status, String> {
        self.poll();
        if let Some(error) = &self.error {
            return Err(error.clone());
        }
        let deadline = Instant::now() + Duration::from_secs(12);
        let result = (|| -> Result<(), Failure> {
            match action {
                "play" => {
                    if self.ended {
                        self.seek_to(0, self.rate, deadline)?;
                        self.ended = false;
                    }
                    self.events
                        .set_state(&self.pipeline, gst::State::Playing, deadline)?;
                    self.playing = true;
                }
                "pause" => {
                    // Like ges-launch/Pitivi, a normal pause is a state request,
                    // not a seek/preroll operation that must finish before ack.
                    self.pipeline
                        .set_state(gst::State::Paused)
                        .map_err(|e| Failure::Pipeline(e.to_string()))?;
                    self.playing = false;
                }
                "seek" => {
                    let target = frame.clamp(0, self.total - 1);
                    if target != seek::displayed_frame(&self.frames) {
                        self.seek_to(target, self.rate, deadline)?;
                    }
                    self.ended = false;
                }
                "rate" => {
                    if !(1..=8).contains(&frame) {
                        return Err(Failure::Pipeline(
                            "播放倍速必须为 0.25 至 2 倍，间隔 0.25".into(),
                        ));
                    }
                    let next = f64::from(frame) / 4.;
                    if next != self.rate {
                        let target = seek::displayed_frame(&self.frames).clamp(0, self.total - 1);
                        self.seek_to(target, next, deadline)?;
                        self.rate = next;
                    }
                }
                "status" => {}
                _ => return Err(Failure::Pipeline("未知播放指令".into())),
            }
            Ok(())
        })();
        if let Err(error) = result {
            let message = error.to_string();
            self.fail(message.clone());
            return Err(message);
        }
        Ok(self.poll())
    }
}
impl Drop for Transport {
    fn drop(&mut self) {
        let _ = self.pipeline.set_state(gst::State::Null);
    }
}
