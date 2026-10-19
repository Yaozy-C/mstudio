#[cfg(target_os = "macos")]
use std::sync::Arc;
#[cfg(test)]
pub(crate) struct ObservedFrame {
    pub position: u32,
    pub red: u8,
    pub rate: f64,
}
#[derive(Clone, Default)]
pub(crate) enum FrameOutput {
    #[default]
    Rgba,
    #[cfg(test)]
    Observed(std::sync::mpsc::Sender<ObservedFrame>),
    #[cfg(target_os = "macos")]
    Shared(Arc<crate::preview_shared::SharedFrames>),
}
impl FrameOutput {
    pub fn format(&self) -> &'static str {
        match self {
            Self::Rgba => "RGBA",
            #[cfg(test)]
            Self::Observed(_) => "RGBA",
            #[cfg(target_os = "macos")]
            Self::Shared(_) => "BGRA",
        }
    }
}
