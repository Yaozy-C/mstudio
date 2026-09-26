//! Render a local test fixture through the production export pipeline.
use mstudio::model::{Asset, RenderSpec};
fn main() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args().collect();
    let fixture: serde_json::Value = serde_json::from_slice(&std::fs::read(&args[1])?)?;
    let spec: RenderSpec = serde_json::from_value(fixture["spec"].clone())?;
    let assets: Vec<Asset> = serde_json::from_value(fixture["assets"].clone())?;
    mstudio::composition::render(
        &spec,
        &assets,
        std::path::Path::new(&args[2]),
        std::path::Path::new(&args[3]),
        |_, _| {},
    )
}
