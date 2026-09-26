use mstudio::{
    model::{Asset, RenderSpec},
    preview_mlt,
};
fn main() -> anyhow::Result<()> {
    let input = std::fs::read_to_string(std::env::args().nth(1).expect("fixture.json"))?;
    let value: serde_json::Value = serde_json::from_str(&input)?;
    let mut spec: RenderSpec = serde_json::from_value(value["spec"].clone())?;
    let mut assets: Vec<Asset> = serde_json::from_value(value["assets"].clone())?;
    if let Some(cache) = std::env::args().nth(2) {
        mstudio::preview_audio::prepare(&mut spec, &mut assets, std::path::Path::new(&cache))?;
    }
    print!("{}", preview_mlt::graph(&spec, &assets)?);
    Ok(())
}
