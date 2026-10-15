//! Drive GLib and the bus while one GES operation is in flight.
use ges::prelude::*;
use gstreamer as gst;
use gstreamer_editing_services as ges;
use std::time::{Duration, Instant};

#[derive(Debug)]
pub(super) enum Failure {
    Timeout,
    Pipeline(String),
}
impl std::fmt::Display for Failure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => f.write_str("预览操作等待超时，请重新加载预览"),
            Self::Pipeline(message) => f.write_str(message),
        }
    }
}
#[derive(Default)]
pub(super) struct Events {
    pub done: u64,
    pub eos: bool,
}
impl Events {
    pub fn poll(&mut self, pipeline: &ges::Pipeline) -> Result<(), Failure> {
        let context = ges::glib::MainContext::ref_thread_default();
        while context.pending() {
            context.iteration(false);
        }
        for message in pipeline.bus().unwrap().iter() {
            match message.view() {
                gst::MessageView::Error(e) => return Err(Failure::Pipeline(e.error().to_string())),
                gst::MessageView::Eos(_) => self.eos = true,
                gst::MessageView::AsyncDone(_) if message.src() == Some(pipeline.upcast_ref()) => {
                    self.done = self.done.wrapping_add(1);
                }
                _ => {}
            }
        }
        Ok(())
    }
    pub fn wait(
        &mut self,
        pipeline: &ges::Pipeline,
        after: Option<u64>,
        deadline: Instant,
        ready: impl Fn() -> bool,
    ) -> Result<(), Failure> {
        loop {
            self.poll(pipeline)?;
            let (result, _, pending) = pipeline.state(gst::ClockTime::ZERO);
            if result.is_err() {
                return Err(Failure::Pipeline("预览状态切换失败".into()));
            }
            if pending == gst::State::VoidPending
                && after.is_none_or(|previous| self.done != previous)
                && ready()
            {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(Failure::Timeout);
            }
            std::thread::sleep(Duration::from_millis(1));
        }
    }
    pub fn set_state(
        &mut self,
        pipeline: &ges::Pipeline,
        state: gst::State,
        deadline: Instant,
    ) -> Result<(), Failure> {
        self.poll(pipeline)?;
        let before = self.done;
        let result = pipeline
            .set_state(state)
            .map_err(|e| Failure::Pipeline(e.to_string()))?;
        let after = (result == gst::StateChangeSuccess::Async).then_some(before);
        self.wait(pipeline, after, deadline, || {
            pipeline.state(gst::ClockTime::ZERO).1 == state
        })
    }
}
