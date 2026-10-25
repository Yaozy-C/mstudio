//! Opt-in isolation of the exact native VAE without any DiT restoration step.
#[cfg(all(target_os = "macos", not(test)))]
#[path = "../src/tensor.rs"]
#[allow(dead_code)]
mod tensor;
#[cfg(all(target_os = "macos", not(test)))]
#[path = "../src/tiling.rs"]
#[allow(dead_code)]
mod tiling;
#[cfg(all(target_os = "macos", not(test)))]
#[path = "../src/vae/mod.rs"]
mod vae;

#[cfg(all(target_os = "macos", not(test)))]
fn main() -> anyhow::Result<()> {
    use anyhow::{Context, ensure};
    use mlx_gen::weights::Weights;
    use mlx_rs::{Array, Dtype};
    use std::{path::PathBuf, time::Instant};
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    ensure!(args.len() == 4, "SOURCE MODELS OUTPUT TILE");
    let source = PathBuf::from(&args[0]);
    let models = PathBuf::from(&args[1]);
    let output = PathBuf::from(&args[2]);
    let tile: i32 = args[3].to_str().context("tile UTF8")?.parse()?;
    ensure!(tile >= 256 && tile % 64 == 0, "invalid tile");
    std::fs::create_dir(&output)?;
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    mstudio_image_runtime::initialize(mstudio_image_runtime::built_metallib())?;
    mlx_rs::Device::set_default(&mlx_rs::Device::gpu());
    let image = image::open(&source)?.to_rgb8();
    let (w, h) = image.dimensions();
    let (pw, ph) = (w.div_ceil(16) * 16, h.div_ceil(16) * 16);
    let mut pixels = vec![-1.0f32; pw as usize * ph as usize * 3];
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                pixels[(y as usize * pw as usize + x as usize) * 3 + c] =
                    image.get_pixel(x, y)[c] as f32 / 127.5 - 1.0;
            }
        }
    }
    let started = Instant::now();
    let result = tensor::run_inference(|| {
        let latent = tensor::phase("vae_encode", || {
            let input = Array::from_slice(&pixels, &[1, ph as i32, pw as i32, 3])
                .as_dtype(Dtype::Float16)?;
            input.eval()?;
            let mut weights = Weights::from_file(models.join("seedvr2_ema_vae_fp16.safetensors"))?;
            weights.cast_all(Dtype::Float16)?;
            let encoder = vae::Encoder::load(&weights)?;
            weights.materialize_accessed()?;
            drop(weights);
            tiling::run(&input, tile, tile / 4, 0.125, 16, |x| encoder.forward(x))
        })?;
        let decoded = tensor::phase("vae_decode", || {
            let mut weights = Weights::from_file(models.join("seedvr2_ema_vae_fp16.safetensors"))?;
            weights.cast_all(Dtype::Float16)?;
            let decoder = vae::Decoder::load(&weights)?;
            weights.materialize_accessed()?;
            drop(weights);
            tiling::run_host(&latent, tile / 8, tile / 32, 8.0, 3, |x| decoder.forward(x))
        })?;
        decoded.rgb8(w, h)
    })?;
    result.save(output.join("output.png"))?;
    let report = serde_json::json!({"source":source,"tile":tile,"dtype":"float16","device":"Metal","mode":"VAE encode and decode only; no DiT","seconds":started.elapsed().as_secs_f64(),"size":[w,h],"oneTileCoversWholeImage": tile as u32>=pw && tile as u32>=ph});
    std::fs::write(
        output.join("report.json"),
        serde_json::to_vec_pretty(&report)?,
    )?;
    println!("{report}");
    Ok(())
}
#[cfg(all(not(target_os = "macos"), not(test)))]
fn main() {
    eprintln!("This opt-in GPU diagnostic requires macOS.");
}
