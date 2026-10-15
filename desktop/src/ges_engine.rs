//! All GES objects and control operations stay on one worker.
use gstreamer_editing_services as ges;
use mstudio::preview_ges::Plan;
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex, mpsc};
use std::time::Duration;
#[path = "ges_async.rs"]
mod asynchronous;
#[path = "ges_pipeline.rs"]
mod pipeline;
#[path = "ges_seek.rs"]
mod seek;
#[path = "ges_transport.rs"]
mod transport;
use seek::{displayed_frame, time};
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
fn total_frames(plan: &Plan) -> i32 {
    mstudio::preview_clock::frame_count(plan.duration, plan.fps)
        .saturating_sub(if cfg!(target_os = "macos") { 2 } else { 0 })
        .max(1) as i32
}
pub struct Player {
    tx: mpsc::Sender<(String, i32, Reply)>,
    frames: Arc<Mutex<Vec<u8>>>,
    status: Arc<Mutex<Status>>,
    join: Option<std::thread::JoinHandle<()>>,
}
impl Player {
    pub fn open(plan: Plan) -> Result<Self, String> {
        Self::open_with_output(plan, crate::ges_output::FrameOutput::Rgba)
    }
    pub fn open_with_output(
        plan: Plan,
        output: crate::ges_output::FrameOutput,
    ) -> Result<Self, String> {
        let (tx, rx) = mpsc::channel::<(String, i32, Reply)>();
        let (ready_tx, ready_rx) = mpsc::channel();
        let frames = Arc::new(Mutex::new(Vec::new()));
        let copy = frames.clone();
        let status = Arc::new(Mutex::new(Status {
            total: total_frames(&plan),
            rate: 1.,
            phase: "loading".into(),
            ..Status::default()
        }));
        let state = status.clone();
        let join = std::thread::spawn(move || {
            let context = ges::glib::MainContext::default();
            let _ = context.with_thread_default(|| {
                let mut transport = match transport::Transport::open(&plan, copy, output) {
                    Ok(transport) => transport,
                    Err(error) => {
                        let _ = ready_tx.send(Err(error));
                        return;
                    }
                };
                *state.lock().unwrap() = transport.snapshot();
                let _ = ready_tx.send(Ok(()));
                loop {
                    *state.lock().unwrap() = transport.poll();
                    let (action, frame, reply) = match rx.recv_timeout(Duration::from_millis(10)) {
                        Ok(value) => value,
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
                    let result = transport.control(&action, frame);
                    *state.lock().unwrap() = transport.snapshot();
                    let _ = reply.send(result);
                }
                drop(transport);
                // Finish disposal before releasing the owning GLib context.
                while context.pending() {
                    context.iteration(false);
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
        status.frame = if status.phase == "ended" {
            (status.total - 1).max(0)
        } else {
            displayed_frame(&self.frames).clamp(0, (status.total - 1).max(0))
        };
        status
    }
    pub fn frame(&self, last: u32) -> Vec<u8> {
        let frame = self.frames.lock().unwrap();
        if frame.len() < 16 || u32::from_le_bytes(frame[..4].try_into().unwrap()) == last {
            vec![]
        } else {
            frame.clone()
        }
    }
}
impl Drop for Player {
    fn drop(&mut self) {
        let _ = self.control("close", 0);
        if let Some(thread) = self.join.take() {
            let _ = thread.join();
        }
    }
}
#[cfg(test)]
#[path = "ges_engine_tests.rs"]
mod tests;
