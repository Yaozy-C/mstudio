//! Exercise rapid seek/play/pause commands against a prepared GES plan.
#[path = "../src/ges_engine.rs"]
mod ges_engine;
#[path = "../src/ges_runtime.rs"]
mod ges_runtime;
fn main() -> anyhow::Result<()> {
    ges_runtime::configure();
    let path = std::env::args()
        .nth(1)
        .ok_or_else(|| anyhow::anyhow!("ges_seek_stress plan.json"))?;
    let plan: mstudio::preview_ges::Plan = serde_json::from_slice(&std::fs::read(path)?)?;
    let total = (plan.duration * plan.fps as f64).ceil() as i32;
    anyhow::ensure!(total > 0, "empty timeline");
    let p = ges_engine::Player::open(plan).map_err(anyhow::Error::msg)?;
    for i in 0..100 {
        for fraction in [0.58, 0.6, 0.62, 0., 1., 0.6, 0.15, 0.62] {
            let frame = ((total - 1) as f64 * fraction).round() as i32;
            p.control("pause", 0).map_err(anyhow::Error::msg)?;
            p.control("seek", frame)
                .map_err(|e| anyhow::anyhow!("iteration {i} frame {frame}: {e}"))?;
            let _ = p.frame(0);
            p.control("play", 0).map_err(anyhow::Error::msg)?;
        }
    }
    println!("800 rapid seeks passed");
    Ok(())
}
