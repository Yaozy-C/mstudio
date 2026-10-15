use serde::{Deserialize, Serialize};
#[derive(Clone, Deserialize)]
#[cfg_attr(not(target_os = "macos"), allow(dead_code))]
#[serde(rename_all = "camelCase")]
pub struct Bounds {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
    pub viewport_width: f64,
    pub viewport_height: f64,
    pub visible: bool,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FrameInfo {
    pub sequence: u32,
    pub position: u32,
    pub generation: u64,
}
