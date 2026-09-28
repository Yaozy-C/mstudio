#[path = "../src/ges_engine.rs"]
mod ges_engine;
#[path = "../src/ges_runtime.rs"]
mod ges_runtime;
fn main() -> anyhow::Result<()> {
    ges_runtime::configure();
    let path = std::env::args().nth(1).expect("ges_play plan.json");
    let plan: mstudio::preview_ges::Plan = serde_json::from_slice(&std::fs::read(path)?)?;
    let p = ges_engine::Player::open(plan).map_err(anyhow::Error::msg)?;
    p.control("play", 0).map_err(anyhow::Error::msg)?;
    let start = std::time::Instant::now();
    let mut last = 0;
    let mut count = 0;
    loop {
        let status = p.control("status", 0).map_err(anyhow::Error::msg)?;
        let bytes = p.frame(last);
        if bytes.len() > 16 {
            last = u32::from_le_bytes(bytes[..4].try_into()?);
            count += 1;
        }
        if !status.playing && status.frame == status.total - 1 {
            println!(
                "frames={count}, elapsed={:.2}s, end={}/{}",
                start.elapsed().as_secs_f64(),
                status.frame,
                status.total
            );
            break;
        }
        anyhow::ensure!(start.elapsed().as_secs() < 120, "playback timeout");
        std::thread::sleep(std::time::Duration::from_millis(8));
    }
    for target in [0, 30, 90, 180, 300, 600] {
        p.control("seek", target).map_err(anyhow::Error::msg)?;
        std::thread::sleep(std::time::Duration::from_millis(150));
        let s = p.control("status", 0).map_err(anyhow::Error::msg)?;
        println!("seek {target}: {}", s.frame);
    }
    Ok(())
}
