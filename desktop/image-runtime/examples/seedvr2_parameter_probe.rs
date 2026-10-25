//! Opt-in fixed-weight parameter isolation. Production restoration is unchanged.
#![cfg_attr(test, allow(dead_code, unused_imports))]
#[cfg(all(target_os = "macos", not(test)))]
#[path = "../src/tensor.rs"]
mod tensor;
#[cfg(all(target_os = "macos", not(test)))]
#[path = "../src/tiling.rs"]
mod tiling;
#[cfg(all(target_os = "macos", not(test)))]
#[path = "../src/vae/mod.rs"]
mod vae;
#[cfg(all(target_os = "macos", not(test)))]
use mstudio_image_runtime::{Device, RestoreOptions};
#[cfg(all(target_os = "macos", not(test)))]
#[derive(Default)]
struct Probe {
    dit_fp32: bool,
    vae_fp32: bool,
    edge_padding: bool,
    fp32_rope: bool,
}
#[cfg(all(target_os = "macos", not(test)))]
fn load_image(path: &std::path::Path) -> anyhow::Result<image::RgbImage> {
    use image::ImageDecoder;
    let mut decoder = image::ImageReader::open(path)?.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = image::DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(image.into_rgb8())
}
#[cfg(all(target_os = "macos", not(test)))]
mod probe_impl {
    use super::vae::{Decoder, Encoder};
    use super::{Device, Probe, RestoreOptions, load_image, tensor, tiling};
    use anyhow::{Result, ensure};
    use image::imageops::{self, FilterType};
    use mlx_gen::weights::Weights;
    use mlx_gen_seedvr2::{config::DitConfig, convert, dit::Seedvr2Transformer};
    use mlx_rs::{Array, Dtype, ops, random};
    use std::path::{Path, PathBuf};

    pub const TRANSFORMER: &str = "seedvr2_distill_6L_1.4B_sharp_fp16_comfyui.safetensors";
    pub const VAE: &str = "seedvr2_ema_vae_fp16.safetensors";

    pub(super) fn run_probe(
        source: &Path,
        directory: &Path,
        device: Device,
        options: RestoreOptions,
        output: &Path,
        probe: &Probe,
    ) -> Result<Vec<PathBuf>> {
        let RestoreOptions {
            scale,
            seed,
            vae_tile: tile,
            quantize8,
            ..
        } = options;
        let input = load_image(source)?;
        let width = input
            .width()
            .checked_mul(scale)
            .ok_or_else(|| anyhow::anyhow!("Image width overflow"))?;
        let height = input
            .height()
            .checked_mul(scale)
            .ok_or_else(|| anyhow::anyhow!("Image height overflow"))?;
        ensure!(
            width <= i32::MAX as u32 - 16 && height <= i32::MAX as u32 - 16,
            "Image dimensions exceed MLX limits"
        );
        let ph = height.div_ceil(16) * 16;
        let pw = width.div_ceil(16) * 16;
        let dtype = if device == Device::Metal && !probe.vae_fp32 {
            Dtype::Float16
        } else {
            Dtype::Float32
        };

        let dit_dtype = if probe.dit_fp32 {
            Dtype::Float32
        } else {
            dtype
        };

        // Each model and its transient tensors die inside its evaluated phase.
        let latent = tensor::phase("vae_encode", || {
            let resized = imageops::resize(&input, width, height, FilterType::Lanczos3);
            drop(input);
            let mut pixels = vec![-1f32; ph as usize * pw as usize * 3];
            for y in 0..height {
                for x in 0..width {
                    for c in 0..3 {
                        pixels[(y as usize * pw as usize + x as usize) * 3 + c] =
                            resized.get_pixel(x, y)[c] as f32 / 127.5 - 1.;
                    }
                }
            }
            if probe.edge_padding {
                for y in 0..ph {
                    for x in 0..pw {
                        if y >= height || x >= width {
                            for c in 0..3 {
                                pixels[(y as usize * pw as usize + x as usize) * 3 + c] = resized
                                    .get_pixel(x.min(width - 1), y.min(height - 1))[c]
                                    as f32
                                    / 127.5
                                    - 1.;
                            }
                        }
                    }
                }
            }
            drop(resized);
            let input =
                Array::from_slice(&pixels, &[1, ph as i32, pw as i32, 3]).as_dtype(dtype)?;
            input.eval()?;
            drop(pixels);
            let mut raw = Weights::from_file(directory.join(VAE))?;
            raw.cast_all(dtype)?;
            let encoder = Encoder::load(&raw)?;
            raw.materialize_accessed()?;
            drop(raw);
            tiling::run(&input, tile as i32, (tile / 4) as i32, 0.125, 16, |x| {
                encoder.forward(x)
            })
        })?;

        Array::save_safetensors(
            [("latent", &latent)],
            None::<&std::collections::HashMap<String, String>>,
            output.join("encoded.safetensors"),
        )?;
        let restored = tensor::phase("dit", || {
            let raw = Weights::from_file(directory.join(TRANSFORMER))?;
            let mut weights = convert::convert_dit(&raw)?;
            weights.cast_all(dit_dtype)?;
            if probe.fp32_rope {
                let keys: Vec<String> = weights
                    .keys()
                    .filter(|k| k.ends_with(".attn.rope.freqs"))
                    .map(str::to_owned)
                    .collect();
                for key in keys {
                    let n = weights.require(&key)?.shape()[0];
                    let values: Vec<f32> = (0..n)
                        .map(|i| (1. + 127. * i as f32 / (n - 1) as f32) * std::f32::consts::PI)
                        .collect();
                    weights.insert(key, Array::from_slice(&values, &[n]));
                }
            }
            let config = DitConfig {
                num_layers: 6,
                mm_layers: 6,
                ..DitConfig::seedvr2_7b()
            };
            let mut transformer = Seedvr2Transformer::from_weights(&weights, &config)?;
            let conditioning = weights.require("positive_conditioning")?.expand_dims(0)?;
            weights.materialize_accessed()?;
            drop(weights);
            drop(raw);
            // Release dense loader aliases before packing. This is opt-in and may alter output.
            if quantize8 {
                transformer.quantize(8)?;
            }
            let nhwc = latent.shape();
            let mask = Array::ones::<f32>(&[1, 1, 1, nhwc[1], nhwc[2]])?.as_dtype(dit_dtype)?;
            let latent = latent
                .transpose_axes(&[0, 3, 1, 2])?
                .expand_dims(2)?
                .as_dtype(dit_dtype)?;
            let key = random::key(seed)?;
            let noise = random::normal::<f32>(latent.shape(), None, None, Some(&key))?
                .as_dtype(dit_dtype)?;
            Array::save_safetensors(
                [("noise", &noise)],
                None::<&std::collections::HashMap<String, String>>,
                output.join("noise.safetensors"),
            )?;
            let model_input = ops::concatenate_axis(&[&noise, &latent, &mask], 1)?;
            let prediction =
                transformer.forward(&model_input, &conditioning, &Array::from_f32(1000.))?;
            let restored = ops::subtract(&noise, prediction)?
                .squeeze_axes(&[2])?
                .transpose_axes(&[0, 2, 3, 1])?
                .as_dtype(dtype)?;
            restored.eval()?;
            Ok(restored)
        })?;
        drop(latent);
        Array::save_safetensors(
            [("latent", &restored)],
            None::<&std::collections::HashMap<String, String>>,
            output.join("restored.safetensors"),
        )?;

        let decoded = tensor::phase("vae_decode", || {
            let mut raw = Weights::from_file(directory.join(VAE))?;
            raw.cast_all(dtype)?;
            let decoder = Decoder::load(&raw)?;
            raw.materialize_accessed()?;
            drop(raw);
            let decoded = tiling::run_host(
                &restored,
                (tile / 8) as i32,
                (tile / 32) as i32,
                8.,
                3,
                |x| decoder.forward(x),
            )?;
            drop(restored);
            Ok(decoded)
        })?;
        // The FP32 blend buffer is already on the CPU. Crop and quantize directly to RGB8.
        let result = decoded.rgb8(width, height)?;
        drop(decoded);
        let path = output.join("output.png");
        result.save(&path)?;
        Ok(vec![path])
    }
}
#[cfg(all(target_os = "macos", not(test)))]
fn main() -> anyhow::Result<()> {
    use anyhow::ensure;
    let args: Vec<String> = std::env::args().skip(1).collect();
    ensure!(args.len() >= 4, "SOURCE MODELS OUTPUT CASE...");
    mstudio_image_runtime::initialize(mstudio_image_runtime::built_metallib())?;
    mlx_rs::Device::set_default(&mlx_rs::Device::gpu());
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .init();
    std::fs::create_dir_all(&args[2])?;
    for case in &args[3..] {
        let mut p = Probe::default();
        let (mut seed, mut scale) = (42, 1);
        match case.as_str() {
            "baseline" => {}
            "dit-fp32" => p.dit_fp32 = true,
            "fp32-rope" => p.fp32_rope = true,
            "dit-fp32-rope" => {
                p.dit_fp32 = true;
                p.fp32_rope = true;
            }
            "all-fp32" => {
                p.dit_fp32 = true;
                p.vae_fp32 = true;
            }
            "edge-padding" => p.edge_padding = true,
            "seed-0" => seed = 0,
            "seed-43" => seed = 43,
            "scale-2" => scale = 2,
            "scale-4" => scale = 4,
            _ => anyhow::bail!("Unknown case {case}"),
        }
        let dir = std::path::Path::new(&args[2]).join(case);
        std::fs::create_dir(&dir)?;
        let started = std::time::Instant::now();
        println!("START {case}");
        let result = tensor::run_inference(|| {
            probe_impl::run_probe(
                std::path::Path::new(&args[0]),
                std::path::Path::new(&args[1]),
                Device::Metal,
                RestoreOptions {
                    scale,
                    seed,
                    denoise: 1.0,
                    vae_tile: 512,
                    quantize8: false,
                    color_correction: mstudio_image_runtime::ColorCorrection::None,
                },
                &dir,
                &p,
            )
        });
        let report = serde_json::json!({"case":case,"source":args[0],"seed":seed,"scale":scale,"tile":512,"q8":false,"ditFp32":p.dit_fp32,"vaeFp32":p.vae_fp32,"edgePadding":p.edge_padding,"fp32Rope":p.fp32_rope,"seconds":started.elapsed().as_secs_f64(),"error":result.as_ref().err().map(ToString::to_string)});
        std::fs::write(dir.join("report.json"), serde_json::to_vec_pretty(&report)?)?;
        println!("RESULT {report}");
        result?;
    }
    Ok(())
}
#[cfg(all(not(target_os = "macos"), not(test)))]
fn main() {
    eprintln!("Requires macOS/MLX");
}
