#[cfg(target_os = "macos")]
use std::sync::Arc;
#[derive(Clone, Default)]
pub(crate) enum FrameOutput {
    #[default]
    Rgba,
    #[cfg(test)]
    Observed(std::sync::mpsc::Sender<(u32, u8)>),
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
