#[cfg(target_os = "macos")]
use std::sync::Arc;
#[derive(Clone, Default)]
pub(crate) enum FrameOutput {
    #[default]
    Rgba,
    #[cfg(target_os = "macos")]
    Shared(Arc<crate::preview_shared::SharedFrames>),
}
impl FrameOutput {
    pub fn format(&self) -> &'static str {
        match self {
            Self::Rgba => "RGBA",
            #[cfg(target_os = "macos")]
            Self::Shared(_) => "BGRA",
        }
    }
}
