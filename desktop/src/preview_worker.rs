//! Runs before Tauri initialization, in an independently crashable process.
use crate::{
    ges_engine::Player,
    preview_protocol::{self as ipc, Event, Request},
};
use std::{
    io::{BufReader, stdin, stdout},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, Ordering},
    },
    time::Duration,
};
pub fn run() -> Result<(), String> {
    let mut input = BufReader::new(stdin());
    let (request, _) = ipc::read(&mut input).map_err(|e| e.to_string())?;
    let (plan, frames) = match request {
        Request::Open(plan) => (plan, crate::ges_output::FrameOutput::Rgba),
        #[cfg(target_os = "macos")]
        Request::OpenMetal { plan, frames } => (
            plan,
            crate::ges_output::FrameOutput::Shared(Arc::new(
                crate::preview_shared::SharedFrames::open(frames)?,
            )),
        ),
        _ => return Err("Preview worker requires an open request".into()),
    };
    let output = Arc::new(Mutex::new(stdout()));
    let opened = match frames {
        crate::ges_output::FrameOutput::Rgba => Player::open(plan),
        #[cfg(any(target_os = "macos", test))]
        output => Player::open_with_output(plan, output),
    };
    let player = match opened {
        Ok(player) => Arc::new(player),
        Err(error) => {
            let _ = ipc::write(
                &mut *output.lock().unwrap(),
                &Event::Ready(Err(error.clone())),
                &[],
            );
            return Err(error);
        }
    };
    ipc::write(
        &mut *output.lock().unwrap(),
        &Event::Ready(Ok(player.snapshot())),
        &[],
    )
    .map_err(|e| e.to_string())?;
    let running = Arc::new(AtomicBool::new(true));
    let control_player = player.clone();
    let control_output = output.clone();
    let control_running = running.clone();
    let commands = std::thread::spawn(move || {
        while let Ok((Request::Control { id, command }, _)) = ipc::read(&mut input) {
            let close = matches!(command, ipc::Command::Close);
            let result = command
                .engine_args()
                .and_then(|(action, frame)| control_player.control(action, frame));
            if ipc::write(
                &mut *control_output.lock().unwrap(),
                &Event::Reply { id, result },
                &[],
            )
            .is_err()
                || close
            {
                break;
            }
        }
        control_running.store(false, Ordering::Release);
    });
    let mut last = 0;
    while running.load(Ordering::Acquire) {
        // Capture while holding the writer lock so an older snapshot cannot
        // be emitted after a newer command acknowledgement.
        let mut writer = output.lock().unwrap();
        let status = player.snapshot();
        let frame = if matches!(status.phase.as_str(), "paused" | "playing" | "ended") {
            player.frame(last)
        } else {
            vec![]
        };
        if frame.len() >= 16 {
            last = u32::from_le_bytes(frame[..4].try_into().unwrap());
        }
        if ipc::write(&mut *writer, &Event::Snapshot(status), &frame).is_err() {
            // Parent closed the pipe. Exit immediately, even if a plugin is stuck.
            std::process::exit(0);
        }
        drop(writer);
        std::thread::sleep(Duration::from_millis(16));
    }
    let _ = commands.join();
    // Do not let a stuck plugin destructor orphan a worker after its parent exits.
    std::process::exit(0)
}
