//! Relocated-bundle smoke test: real import and GES decoding using private tools.
#[path = "../src/ges_engine.rs"]
mod ges_engine;
#[path = "../src/ges_runtime.rs"]
mod ges_runtime;
use mstudio::{
    media,
    preview_ges::{Layer, Plan},
};
use std::path::Path;
fn main() -> anyhow::Result<()> {
    ges_runtime::configure();
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(args.len() == 3, "bundled_media source.mp4 work-directory");
    #[cfg(not(windows))]
    let tools = std::env::current_exe()?
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("Resources/gstreamer/bin");
    #[cfg(windows)]
    let tools = std::env::current_exe()?.parent().unwrap().join("media");
    for name in ["ffmpeg", "ffprobe"] {
        let filename = if cfg!(windows) {
            format!("{name}.exe")
        } else {
            name.into()
        };
        anyhow::ensure!(
            media::binary(name) == tools.join(filename),
            "external media tool"
        );
    }
    let asset = media::import(Path::new(&args[1]), Path::new(&args[2]))?;
    anyhow::ensure!(
        asset.width == 320 && asset.height == 180 && asset.has_audio,
        "unexpected imported asset"
    );
    anyhow::ensure!(Path::new(&asset.preview).is_file(), "missing thumbnail");
    // Exercise LUT paths with drive letters, spaces and Unicode on Windows.
    let visual: mstudio::visual::Visual = serde_json::from_value(serde_json::json!({
        "grade": {"exposure": 0.4}
    }))?;
    let filter = mstudio::visual::ffmpeg(Some(&visual), 320, 180)?;
    media::run(
        media::command("ffmpeg")
            .args(["-v", "error", "-y", "-i"])
            .arg(&asset.path)
            .args(["-vf", &filter, "-frames:v", "1"])
            .arg(Path::new(&args[2]).join("graded.png")),
    )?;
    let player = ges_engine::Player::open(Plan {
        width: 320,
        height: 180,
        fps: 24,
        duration: 1.,
        audio: None,
        files: vec![],
        layers: vec![Layer {
            path: asset.path,
            start: 0.,
            duration: 1.,
            trim: 0.,
            x: 0,
            y: 0,
            width: 320,
            height: 180,
            opacity: 1.,
        }],
    })
    .map_err(anyhow::Error::msg)?;
    player.control("seek", 12).map_err(anyhow::Error::msg)?;
    let frame = player.frame(0);
    anyhow::ensure!(frame.len() == 16 + 320 * 180 * 4, "missing GES frame");
    anyhow::ensure!(
        frame[16..]
            .chunks_exact(4)
            .any(|p| p[0] > 30 || p[1] > 30 || p[2] > 30),
        "unexpected black GES frame"
    );
    println!("Private tool resolution, import, thumbnail and GES seek/decode passed");
    Ok(())
}
