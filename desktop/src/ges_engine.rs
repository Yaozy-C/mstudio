//! All GES objects stay on one worker. Only bounded RGBA snapshots cross threads.
use ges::prelude::*;
use serde::Serialize;
#[path = "ges_pipeline.rs"]
mod pipeline;
use gstreamer as gst;
use gstreamer_editing_services as ges;
use mstudio::preview_ges::Plan;
use pipeline::build;
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
#[derive(Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct Status {
    pub frame: i32,
    pub playing: bool,
    pub total: i32,
}
type Reply = mpsc::Sender<Result<Status, String>>;
pub struct Player {
    tx: mpsc::Sender<(String, i32, Reply)>,
    frames: Arc<Mutex<Vec<u8>>>,
    join: Option<std::thread::JoinHandle<()>>,
}
fn time(s: f64) -> gst::ClockTime {
    gst::ClockTime::from_nseconds((s * 1e9).round() as u64)
}
impl Player {
    pub fn open(plan: Plan) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<(String, i32, Reply)>();
        let (ready_tx, ready_rx) = mpsc::channel();
        let frames = Arc::new(Mutex::new(Vec::new()));
        let copy = frames.clone();
        let join = std::thread::spawn(move || {
            let context = ges::glib::MainContext::default();
            let _ = context.with_thread_default(|| {
                let result = build(&plan, copy);
                match result {
                    Err(e) => {
                        let _ = ready_tx.send(Err(e.to_string()));
                    }
                    Ok(pipeline) => {
                        let _ = ready_tx.send(Ok(()));
                        let mut playing = false;
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
                            let result = (|| -> Result<Status, String> {
                                if let Some(e) = &error {
                                    return Err(e.clone());
                                }
                                let total = (plan.duration * plan.fps as f64).ceil() as i32;
                                match action.as_str() {
                                    "play" => {
                                        if ended {
                                            pipeline
                                                .seek_simple(
                                                    gst::SeekFlags::FLUSH
                                                        | gst::SeekFlags::ACCURATE,
                                                    gst::ClockTime::ZERO,
                                                )
                                                .map_err(|e| e.to_string())?;
                                            ended = false;
                                        }
                                        pipeline
                                            .set_state(gst::State::Playing)
                                            .map_err(|e| e.to_string())?;
                                        playing = true;
                                    }
                                    "pause" => {
                                        pipeline
                                            .set_state(gst::State::Paused)
                                            .map_err(|e| e.to_string())?;
                                        playing = false;
                                    }
                                    "seek" => {
                                        pipeline
                                            .seek_simple(
                                                gst::SeekFlags::FLUSH | gst::SeekFlags::ACCURATE,
                                                time(
                                                    frame.clamp(0, total - 1) as f64
                                                        / plan.fps as f64,
                                                ),
                                            )
                                            .map_err(|e| e.to_string())?;
                                        ended = false;
                                    }
                                    "status" => {}
                                    _ => return Err("未知播放指令".into()),
                                }
                                let position = pipeline
                                    .query_position::<gst::ClockTime>()
                                    .unwrap_or(gst::ClockTime::ZERO)
                                    .seconds_f64();
                                Ok(Status {
                                    frame: if ended {
                                        total - 1
                                    } else {
                                        (position * plan.fps as f64).round() as i32
                                    },
                                    playing,
                                    total,
                                })
                            })();
                            let _ = reply.send(result);
                        }
                        let _ = pipeline.set_state(gst::State::Null);
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
            join: Some(join),
        })
    }
    pub fn control(&self, action: &str, frame: i32) -> Result<Status, String> {
        let (tx, rx) = mpsc::channel();
        self.tx
            .send((action.into(), frame, tx))
            .map_err(|e| e.to_string())?;
        rx.recv_timeout(Duration::from_secs(15))
            .map_err(|e| e.to_string())?
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
