//! Pixel regression for repeated seeks into a known non-black interval.
#[path = "../src/ges_engine.rs"]
mod ges_engine;
#[path = "../src/ges_runtime.rs"]
mod ges_runtime;
fn main() -> anyhow::Result<()> {
    ges_runtime::configure();
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(
        args.len() == 4,
        "ges_seek_pixels plan.json first-frame last-frame"
    );
    let plan: mstudio::preview_ges::Plan = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let first: i32 = args[2].parse()?;
    let last: i32 = args[3].parse()?;
    let p = ges_engine::Player::open(plan).map_err(anyhow::Error::msg)?;
    for cycle in 0..10 {
        for target in (first..=last).chain((first..=last).rev()) {
            p.control("pause", 0).map_err(anyhow::Error::msg)?;
            p.control("seek", target).map_err(anyhow::Error::msg)?;
            let f = p.frame(0);
            let mean: f64 = f[16..]
                .chunks_exact(4)
                .map(|p| (p[0] as f64 + p[1] as f64 + p[2] as f64) / 3.)
                .sum::<f64>()
                / ((f.len() - 16) / 4) as f64;
            anyhow::ensure!(
                mean > 10.,
                "black frame: cycle={cycle} target={target} mean={mean}"
            );
            p.control("play", 0).map_err(anyhow::Error::msg)?;
        }
        println!("cycle {cycle}: no black frames");
    }
    Ok(())
}
