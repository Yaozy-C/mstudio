//! Read one paused frame through the production GES player for pixel regression tests.
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
    anyhow::ensure!(args.len() == 4, "ges_frame plan.json seconds output.ppm");
    let plan: mstudio::preview_ges::Plan = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let target = (args[2].parse::<f64>()? * plan.fps as f64).round() as i32;
    anyhow::ensure!(
        target >= 0 && target < (plan.duration * plan.fps as f64).ceil() as i32,
        "frame outside timeline"
    );
    let player = ges_engine::Player::open(plan).map_err(anyhow::Error::msg)?;
    player.control("seek", target).map_err(anyhow::Error::msg)?;
    let deadline = Instant::now() + Duration::from_secs(15);
    loop {
        let bytes = player.frame(0);
        if bytes.len() > 16 && u32::from_le_bytes(bytes[12..16].try_into()?) == target as u32 {
            let width = u32::from_le_bytes(bytes[4..8].try_into()?);
            let height = u32::from_le_bytes(bytes[8..12].try_into()?);
            let mut file = std::fs::File::create(&args[3])?;
            write!(file, "P6\n{width} {height}\n255\n")?;
            let rgb: Vec<_> = bytes[16..]
                .chunks_exact(4)
                .flat_map(|p| p[..3].iter().copied())
                .collect();
            file.write_all(&rgb)?;
            break;
        }
        player.control("status", 0).map_err(anyhow::Error::msg)?;
        anyhow::ensure!(Instant::now() < deadline, "GES frame timed out");
        std::thread::sleep(Duration::from_millis(10));
    }
    Ok(())
}
