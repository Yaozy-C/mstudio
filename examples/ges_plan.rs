//! Prepare a read-only project fixture for GES playback validation.
use mstudio::{
    model::{Asset, RenderSpec},
    preview_ges,
};
use serde::Deserialize;
#[derive(Deserialize)]
struct Fixture {
    spec: RenderSpec,
    assets: Vec<Asset>,
}
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    anyhow::ensure!(args.len() == 3, "ges_plan fixture.json cache-directory");
    let mut f: Fixture = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let root = std::path::Path::new(&args[2]);
    mstudio::transitions::prepare(&mut f.spec, &mut f.assets, root, true)?;
    let plan = preview_ges::prepare(&f.spec, &f.assets, root, 640)?;
    println!("{}", serde_json::to_string_pretty(&plan)?);
    Ok(())
}
