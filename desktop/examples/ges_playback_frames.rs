//! Capture frames during normal playback (seeking alone misses scheduling bugs).
#[path = "../src/ges_engine.rs"]
mod ges_engine;
#[path = "../src/ges_runtime.rs"]
mod ges_runtime;
use std::{
    io::Write,
    time::{Duration, Instant},
};
fn main() -> anyhow::Result<()> {
    ges_runtime::configure();
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        args.len() == 5,
        "ges_playback_frames plan.json start end directory"
    );
    let plan: mstudio::preview_ges::Plan = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let start = (args[2].parse::<f64>()? * plan.fps as f64).round() as i32;
    let end = (args[3].parse::<f64>()? * plan.fps as f64).round() as u32;
    let player = ges_engine::Player::open(plan).map_err(anyhow::Error::msg)?;
    player.control("seek", start).map_err(anyhow::Error::msg)?;
    let deadline = Instant::now() + Duration::from_secs(30);
    loop {
        let bytes = player.frame(0);
        if bytes.len() > 16 && u32::from_le_bytes(bytes[12..16].try_into()?) == start as u32 {
            break;
        }
        anyhow::ensure!(Instant::now() < deadline, "seek timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
    std::fs::create_dir_all(&args[4])?;
    player.control("play", 0).map_err(anyhow::Error::msg)?;
    let mut last = 0;
    loop {
        let bytes = player.frame(last);
        if bytes.len() > 16 {
            last = u32::from_le_bytes(bytes[..4].try_into()?);
            let position = u32::from_le_bytes(bytes[12..16].try_into()?);
            let width = u32::from_le_bytes(bytes[4..8].try_into()?);
            let height = u32::from_le_bytes(bytes[8..12].try_into()?);
            let mut f = std::fs::File::create(format!("{}/{position:06}.ppm", args[4]))?;
            write!(f, "P6\n{width} {height}\n255\n")?;
            let rgb: Vec<_> = bytes[16..]
                .chunks_exact(4)
                .flat_map(|p| p[..3].iter().copied())
                .collect();
            f.write_all(&rgb)?;
            if position >= end {
                break;
            }
        }
        anyhow::ensure!(Instant::now() < deadline, "playback timed out");
        std::thread::sleep(Duration::from_millis(2));
    }
    Ok(())
}
