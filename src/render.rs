pub use crate::composition::render;

pub fn tempo(speed: f64) -> String {
    if speed < 0.5 {
        format!("atempo=0.5,atempo={}", speed / 0.5)
    } else if speed > 2.0 {
        format!("atempo=2,atempo={}", speed / 2.0)
    } else {
        format!("atempo={speed}")
    }
}
