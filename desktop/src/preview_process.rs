//! Parent-side handle. No GStreamer object or global lock crosses a wait.
use crate::{
    ges_engine::Status,
    preview_protocol::{self as ipc, Command, Event, Request},
};
use mstudio::preview_ges::Plan;
use std::{
    collections::HashMap,
    io::BufReader,
    process::{Child, ChildStdin, Stdio},
    sync::{
        Arc, Mutex,
        atomic::{AtomicU64, Ordering},
        mpsc,
    },
    time::{Duration, Instant},
};
type Reply = mpsc::Sender<Result<Status, String>>;
#[derive(Default)]
struct Shared {
    status: Status,
    frame: Arc<Vec<u8>>,
    failure: Option<String>,
    pending: HashMap<u64, Reply>,
}
pub struct PreviewProcess {
    child: Mutex<Child>,
    input: Mutex<ChildStdin>,
    shared: Arc<Mutex<Shared>>,
    next: AtomicU64,
    started: Mutex<Option<mpsc::Receiver<Result<Status, String>>>>,
}
impl PreviewProcess {
    pub fn launch(plan: Plan) -> Result<Self, String> {
        let mut command =
            std::process::Command::new(std::env::current_exe().map_err(|e| e.to_string())?);
        mstudio::media::quiet(&mut command);
        command.arg("--preview-worker");
        Self::spawn(command, plan)
    }
    fn spawn(mut command: std::process::Command, plan: Plan) -> Result<Self, String> {
        let mut child = command
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::inherit())
            .spawn()
            .map_err(|e| e.to_string())?;
        let input = child.stdin.take().ok_or("Preview stdin unavailable")?;
        let mut output = BufReader::new(child.stdout.take().ok_or("Preview stdout unavailable")?);
        let shared = Arc::new(Mutex::new(Shared::default()));
        let copy = shared.clone();
        let (ready, started) = mpsc::channel();
        std::thread::spawn(move || {
            loop {
                match ipc::read::<Event>(&mut output) {
                    Ok((event, frame)) => {
                        let mut state = copy.lock().unwrap();
                        match event {
                            Event::Ready(result) => {
                                if let Ok(status) = &result {
                                    state.status = status.clone();
                                }
                                let _ = ready.send(result);
                            }
                            Event::Snapshot(status) => state.status = status,
                            Event::Reply { id, result } => {
                                if let Ok(status) = &result {
                                    state.status = status.clone();
                                }
                                if let Some(reply) = state.pending.remove(&id) {
                                    let _ = reply.send(result);
                                }
                            }
                        }
                        if !frame.is_empty() {
                            state.frame = Arc::new(frame);
                        }
                    }
                    Err(error) => {
                        fail(
                            &copy,
                            format!("预览进程已退出或连接中断，可重新加载预览：{error}"),
                        );
                        break;
                    }
                }
            }
        });
        let process = Self {
            child: Mutex::new(child),
            input: Mutex::new(input),
            shared,
            next: AtomicU64::new(1),
            started: Mutex::new(Some(started)),
        };
        ipc::write(
            &mut *process.input.lock().unwrap(),
            &Request::Open(plan),
            &[],
        )
        .map_err(|e| e.to_string())?;
        Ok(process)
    }
    pub fn ready(&self) -> Result<(), String> {
        let started = self
            .started
            .lock()
            .unwrap()
            .take()
            .ok_or("预览启动已确认")?;
        match started.recv_timeout(Duration::from_secs(60)) {
            Ok(result) => result.map(|_| ()),
            Err(error) => self.abort(format!("预览进程启动失败：{error}")).map(|_| ()),
        }
    }
    pub fn control(&self, command: Command) -> Result<Status, String> {
        command.engine_args()?;
        let id = self.next.fetch_add(1, Ordering::Relaxed);
        let (reply, response) = mpsc::channel();
        {
            let mut state = self.shared.lock().unwrap();
            if let Some(error) = &state.failure {
                return Err(error.clone());
            }
            state.pending.insert(id, reply);
        }
        if let Err(error) = ipc::write(
            &mut *self.input.lock().unwrap(),
            &Request::Control { id, command },
            &[],
        ) {
            return self.abort(format!("预览指令发送失败：{error}"));
        }
        match response.recv_timeout(Duration::from_secs(15)) {
            Ok(result) => result,
            Err(error) => self.abort(format!("预览操作超时，可重新加载预览：{error}")),
        }
    }
    fn abort(&self, message: String) -> Result<Status, String> {
        fail(&self.shared, message.clone());
        let _ = self.child.lock().unwrap().kill();
        Err(message)
    }
    pub fn snapshot(&self) -> Result<Status, String> {
        let state = self.shared.lock().unwrap();
        if let Some(error) = &state.failure {
            Err(error.clone())
        } else {
            Ok(state.status.clone())
        }
    }
    pub fn frame(&self, last: u32) -> Vec<u8> {
        let state = self.shared.lock().unwrap();
        if state.failure.is_some()
            || !matches!(state.status.phase.as_str(), "paused" | "playing" | "ended")
            || state.frame.len() < 16
            || u32::from_le_bytes(state.frame[..4].try_into().unwrap()) == last
        {
            vec![]
        } else {
            let frame = state.frame.clone();
            drop(state);
            frame.as_ref().clone()
        }
    }
    pub fn shutdown(&self) {
        // Closing supersedes in-flight controls; a stuck plugin gets a bounded grace period.
        fail(&self.shared, "预览已关闭".into());
        let _ = ipc::write(
            &mut *self.input.lock().unwrap(),
            &Request::Control {
                id: 0,
                command: Command::Close,
            },
            &[],
        );
        let deadline = Instant::now() + Duration::from_millis(500);
        let mut child = self.child.lock().unwrap();
        while Instant::now() < deadline {
            if matches!(child.try_wait(), Ok(Some(_))) {
                return;
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        let _ = child.kill();
        let _ = child.wait();
    }
}
fn fail(shared: &Mutex<Shared>, message: String) {
    let mut state = shared.lock().unwrap();
    state.failure = Some(message.clone());
    for (_, reply) in state.pending.drain() {
        let _ = reply.send(Err(message.clone()));
    }
}
impl Drop for PreviewProcess {
    fn drop(&mut self) {
        self.shutdown();
    }
}
#[cfg(test)]
#[path = "preview_process_tests.rs"]
mod tests;
