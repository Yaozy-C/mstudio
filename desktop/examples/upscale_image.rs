//! Exercise the production Real-ESRGAN path without registering project assets.
#[cfg(target_os = "macos")]
fn main() -> anyhow::Result<()> {
    use anyhow::ensure;
    use mstudio_image_runtime::{Device, built_metallib, initialize, upscale};
    use std::{path::PathBuf, time::Instant};
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(args.len() == 5, "SOURCE MODELS OUTPUT SCALE TILE");
    let source = PathBuf::from(&args[0]);
    let models = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let scale = args[3].to_str().unwrap().parse()?;
    let tile = args[4].to_str().unwrap().parse()?;
    std::fs::create_dir(&output)?;
    initialize(built_metallib())?;
    let start = Instant::now();
    let paths = upscale(&source, &models, Device::Metal, scale, tile, &output)?;
    println!(
        "{}",
        serde_json::json!({"scale":scale,"tile":tile,"elapsedSeconds":start.elapsed().as_secs_f64(),"outputs":paths})
    );
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    panic!("Native image inference requires macOS");
}
