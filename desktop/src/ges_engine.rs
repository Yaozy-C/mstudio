//! All GES objects stay on one worker. Only bounded RGBA snapshots cross threads.
use ges::prelude::*;
use serde::{Deserialize, Serialize};
#[path = "ges_pipeline.rs"]
mod pipeline;
use gstreamer as gst;
use gstreamer_editing_services as ges;
use mstudio::preview_ges::Plan;
use pipeline::build;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
#[path = "ges_seek.rs"]
mod seek;
use seek::{displayed_frame, seek, settle, time};
#[derive(Clone, Debug, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub frame: i32,
    pub playing: bool,
    pub total: i32,
    pub phase: String,
    pub rate: f64,
    pub error: Option<String>,
}
type Reply = mpsc::Sender<Result<Status, String>>;
pub struct Player {
    tx: mpsc::Sender<(String, i32, Reply)>,
    frames: Arc<Mutex<Vec<u8>>>,
    status: Arc<Mutex<Status>>,
    join: Option<std::thread::JoinHandle<()>>,
}
impl Player {
    pub fn open(plan: Plan) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<(String, i32, Reply)>();
        let (ready_tx, ready_rx) = mpsc::channel();
        let frames = Arc::new(Mutex::new(Vec::new()));
        let copy = frames.clone();
        let status = Arc::new(Mutex::new(Status {
            total: (plan.duration * plan.fps as f64).ceil() as i32,
            rate: 1.,
            phase: "loading".into(),
            ..Status::default()
        }));
        let state = status.clone();
        let join = std::thread::spawn(move || {
            let context = ges::glib::MainContext::default();
            let _ = context.with_thread_default(|| {
                let result = build(&plan, copy.clone());
                match result {
                    Err(e) => {
                        let _ = ready_tx.send(Err(e.to_string()));
                    }
                    Ok(pipeline) => {
                        let _ = ready_tx.send(Ok(()));
                        let mut playing = false;
                        let mut rate = 1.0;
                        let mut ended = false;
                        let mut error = None;
                        loop {
                            while context.pending() {
                                context.iteration(false);
                            }
                            for msg in pipeline.bus().unwrap().iter() {
                                match msg.view() {
                                    gst::MessageView::Error(e) => {
                                        error = Some(e.error().to_string());
                                        playing = false;
                                        let _ = pipeline.set_state(gst::State::Paused);
                                    }
                                    gst::MessageView::Eos(_) => {
                                        ended = true;
                                        playing = false;
                                        let _ = pipeline.set_state(gst::State::Paused);
                                    }
                                    _ => {}
                                }
                            }
                            *state.lock().unwrap() = Status {
                                frame: displayed_frame(&copy),
                                playing,
                                rate,
                                total: (plan.duration * plan.fps as f64).ceil() as i32,
                                phase: if error.is_some() {
                                    "error"
                                } else if ended {
                                    "ended"
                                } else if playing {
                                    "playing"
                                } else {
                                    "paused"
                                }
                                .into(),
                                error: error.clone(),
                            };
                            let (action, frame, reply) =
                                match rx.recv_timeout(Duration::from_millis(10)) {
                                    Ok(v) => v,
                                    Err(mpsc::RecvTimeoutError::Timeout) => continue,
                                    Err(_) => break,
                                };
                            if action == "close" {
                                let _ = reply.send(Ok(Status::default()));
                                break;
                            }
                            state.lock().unwrap().phase = match action.as_str() {
                                "play" => "starting",
                                "pause" => "pausing",
                                "seek" => "seeking",
                                "rate" => "changingRate",
                                _ => "paused",
                            }
                            .into();
                            let result = (|| -> Result<Status, String> {
                                if let Some(e) = &error {
                                    return Err(e.clone());
                                }
                                let total = (plan.duration * plan.fps as f64).ceil() as i32;
                                match action.as_str() {
                                    "play" => {
                                        if ended {
                                            seek(
                                                &pipeline,
                                                &copy,
                                                0,
                                                plan.fps,
                                                false,
                                                rate,
                                                plan.duration,
                                            )?;
                                            ended = false;
                                        }
                                        pipeline
                                            .set_state(gst::State::Playing)
                                            .map_err(|e| e.to_string())?;
                                        playing = true;
                                    }
                                    "pause" => {
                                        let current = displayed_frame(&copy).clamp(0, total - 1);
                                        seek(
                                            &pipeline,
                                            &copy,
                                            current,
                                            plan.fps,
                                            false,
                                            rate,
                                            plan.duration,
                                        )?;
                                        playing = false;
                                    }
                                    "seek" => {
                                        seek(
                                            &pipeline,
                                            &copy,
                                            frame.clamp(0, total - 1),
                                            plan.fps,
                                            playing,
                                            rate,
                                            plan.duration,
                                        )?;
                                        ended = false;
                                    }
                                    // The integer payload carries quarter-speed steps (1..=8).
                                    "rate" => {
                                        if !(1..=8).contains(&frame) {
                                            return Err(
                                                "播放倍速必须为 0.25 至 2 倍，间隔 0.25".into()
                                            );
                                        }
                                        let next_rate = f64::from(frame) / 4.0;
                                        let current = displayed_frame(&copy).clamp(0, total - 1);
                                        seek(
                                            &pipeline,
                                            &copy,
                                            current,
                                            plan.fps,
                                            playing,
                                            next_rate,
                                            plan.duration,
                                        )?;
                                        rate = next_rate;
                                    }
                                    "status" => {}
                                    _ => return Err("未知播放指令".into()),
                                }
                                if matches!(action.as_str(), "seek" | "pause" | "play") {
                                    settle(&pipeline)?;
                                }
                                Ok(Status {
                                    frame: if ended {
                                        total - 1
                                    } else {
                                        displayed_frame(&copy).clamp(0, total - 1)
                                    },
                                    playing,
                                    total,
                                    rate,
                                    phase: if ended {
                                        "ended"
                                    } else if playing {
                                        "playing"
                                    } else {
                                        "paused"
                                    }
                                    .into(),
                                    error: None,
                                })
                            })();
                            match &result {
                                Ok(value) => *state.lock().unwrap() = value.clone(),
                                Err(message) => {
                                    error = Some(message.clone());
                                    playing = false;
                                }
                            }
                            let _ = reply.send(result);
                        }
                        let _ = pipeline.set_state(gst::State::Null);
                        drop(pipeline);
                        // Complete GLib disposal callbacks before the worker releases its
                        // context; Windows cannot delete media with an open source handle.
                        while context.pending() {
                            context.iteration(false);
                        }
                    }
                }
            });
        });
        ready_rx
            .recv_timeout(Duration::from_secs(60))
            .map_err(|e| format!("GES 启动失败：{e}"))??;
        Ok(Self {
            tx,
            frames,
            status,
            join: Some(join),
        })
    }
    pub fn control(&self, action: &str, frame: i32) -> Result<Status, String> {
        if action == "status" {
            return Ok(self.snapshot());
        }
        if action == "rate" && !(1..=8).contains(&frame) {
            return Err("播放倍速必须为 0.25 至 2 倍，间隔 0.25".into());
        }
        let (tx, rx) = mpsc::channel();
        self.tx
            .send((action.into(), frame, tx))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(15))
            .map_err(|e| e.to_string())?
    }
    pub fn snapshot(&self) -> Status {
        let mut status = self.status.lock().unwrap().clone();
        if status.phase != "ended" {
            status.frame = displayed_frame(&self.frames).clamp(0, (status.total - 1).max(0));
        } else {
            status.frame = (status.total - 1).max(0);
        }
        status
    }
    pub fn frame(&self, last: u32) -> Vec<u8> {
        let f = self.frames.lock().unwrap();
        if f.len() < 16 || u32::from_le_bytes(f[..4].try_into().unwrap()) == last {
            vec![]
        } else {
            f.clone()
        }
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.control("close", 0);
        if let Some(t) = self.join.take() {
            let _ = t.join();
        }
    }
}
#[cfg(test)]
#[path = "ges_engine_tests.rs"]
mod tests;
