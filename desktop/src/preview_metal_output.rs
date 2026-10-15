use super::*;
use crate::{metal_preview::Surface, preview_shared::SharedFrames, preview_surface::FrameInfo};
#[derive(Default)]
struct Gate {
    generation: u64,
    pending: usize,
    closed: bool,
}
pub(super) struct MetalOutput {
    frames: Arc<SharedFrames>,
    surface: Arc<Surface>,
    gate: Mutex<Gate>,
}
pub(super) struct Operation<'a>(&'a Mutex<Gate>);
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        self.0.lock().unwrap().pending -= 1;
    }
}
impl MetalOutput {
    pub fn unlink(&self) {
        self.frames.unlink();
    }
    pub fn new(frames: Arc<SharedFrames>, surface: Arc<Surface>) -> Self {
        Self {
            frames,
            surface,
            gate: Mutex::new(Gate::default()),
        }
    }
    pub fn begin_control(&self) -> Operation<'_> {
        let mut gate = self.gate.lock().unwrap();
        gate.generation = gate.generation.wrapping_add(1);
        gate.pending += 1;
        self.surface.invalidate(gate.generation);
        Operation(&self.gate)
    }
}
impl PreviewProcess {
    pub fn surface(&self) -> Option<Arc<Surface>> {
        self.metal.as_ref().map(|m| m.surface.clone())
    }
    pub fn detach_surface(&self) {
        if let Some(m) = &self.metal {
            let mut gate = m.gate.lock().unwrap();
            gate.closed = true;
            gate.generation = gate.generation.wrapping_add(1);
            m.surface.invalidate(gate.generation);
            drop(gate);
            m.surface.detach();
        }
    }
    pub fn frame_info(&self, last: u32) -> Result<Option<FrameInfo>, String> {
        let m = self.metal.as_ref().ok_or("Metal 预览不可用")?;
        let gate = m.gate.lock().unwrap();
        let state = self.shared.lock().unwrap();
        if let Some(error) = &state.failure {
            return Err(error.clone());
        }
        if gate.closed
            || gate.pending > 0
            || !matches!(state.status.phase.as_str(), "paused" | "playing" | "ended")
            || state.frame.len() != 16
        {
            return Ok(None);
        }
        let sequence = u32::from_le_bytes(state.frame[..4].try_into().unwrap());
        Ok((sequence != last).then(|| FrameInfo {
            sequence,
            position: u32::from_le_bytes(state.frame[12..16].try_into().unwrap()),
            generation: gate.generation,
        }))
    }
    pub fn present(&self, sequence: u32, generation: u64) -> Result<bool, String> {
        let m = self.metal.as_ref().ok_or("Metal 预览不可用")?;
        let gate = m.gate.lock().unwrap();
        if gate.closed || gate.pending > 0 || gate.generation != generation {
            return Ok(false);
        }
        {
            let state = self.shared.lock().unwrap();
            if let Some(error) = &state.failure {
                return Err(error.clone());
            }
            if !matches!(state.status.phase.as_str(), "paused" | "playing" | "ended") {
                return Ok(false);
            }
        }
        drop(gate);
        // Drawable acquisition may wait; the native submission gate checks this
        // generation again immediately before committing, without blocking controls.
        m.frames
            .with_frame(Some(sequence), |frame| {
                m.surface.present(&frame[16..], generation)
            })
            .unwrap_or(Ok(false))
    }
}
