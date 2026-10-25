//! Opt-in real-image comparison through the unchanged production restoration API.
//! cargo run --release --example compare_vae_tiles -- SOURCE MODELS OUTPUT [TILE...]
#[cfg(target_os = "macos")]
use anyhow::{Context, Result, ensure};
#[cfg(target_os = "macos")]
use mstudio_image_runtime::{
    Device, RestoreOptions, built_metallib, initialize, restore_with_options,
};
#[cfg(target_os = "macos")]
use serde_json::json;
#[cfg(target_os = "macos")]
use std::{path::PathBuf, time::Instant};

#[cfg(target_os = "macos")]
fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(args.len() >= 3, "SOURCE MODELS OUTPUT [TILE...]");
    let source = PathBuf::from(&args[0]);
    let models = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let tiles: Vec<u32> = if args.len() > 3 {
        args[3..]
            .iter()
            .map(|s| {
                s.to_str()
                    .context("tile is not UTF-8")?
                    .parse()
                    .context("invalid tile")
            })
            .collect::<Result<_>>()?
    } else {
        vec![256, 384, 512, 2048]
    };
    initialize(built_metallib())?;
    // Keep historical tiling probes uncorrected unless explicitly testing color.
    let color_correction = match std::env::var("MSTUDIO_TEST_COLOR_CORRECTION") {
        Ok(value) => serde_json::from_value(serde_json::Value::String(value))?,
        Err(_) => mstudio_image_runtime::ColorCorrection::None,
    };
    std::fs::create_dir_all(&output)?;
    let header = std::fs::read(&source)?;
    ensure!(
        header.starts_with(b"\x89PNG\r\n\x1a\n") && header.len() >= 24,
        "Probe expects a PNG"
    );
    let input = (
        u32::from_be_bytes(header[16..20].try_into()?),
        u32::from_be_bytes(header[20..24].try_into()?),
    );
    for tile in tiles {
        let directory = output.join(format!("tile-{tile}"));
        std::fs::create_dir(&directory)
            .context("Use a fresh output directory; existing results are never overwritten")?;
        let started = Instant::now();
        println!(
            "START tile={tile} image={}x{} seed=42 scale=1 q8=false",
            input.0, input.1
        );
        let result = restore_with_options(
            &source,
            &models,
            Device::Metal,
            RestoreOptions {
                scale: 1,
                seed: 42,
                denoise: 1.0,
                vae_tile: tile,
                quantize8: false,
                color_correction,
            },
            &directory,
        );
        let report = match result {
            Ok(files) => {
                json!({"status":"complete", "source":source, "inputSize":[input.0,input.1],"model":"seedvr2-3b", "device":"Metal", "dtype":"float16", "tile":tile,"overlap":tile/4,"seed":42,"scale":1,"q8":false,"colorCorrection":color_correction,"seconds":started.elapsed().as_secs_f64(),"outputs":files,"oneTileCoversWholeImage":tile>=input.0.div_ceil(16)*16 && tile>=input.1.div_ceil(16)*16})
            }
            Err(error) => {
                json!({"status":"failed","tile":tile,"seconds":started.elapsed().as_secs_f64(),"error":error.to_string()})
            }
        };
        std::fs::write(
            directory.join("report.json"),
            serde_json::to_vec_pretty(&report)?,
        )?;
        println!("RESULT {report}");
        ensure!(
            report["status"] == "complete",
            "case failed; inspect its report before continuing"
        );
    }
    Ok(())
}

#[cfg(not(target_os = "macos"))]
fn main() {
    eprintln!("This opt-in GPU diagnostic requires macOS.");
}
