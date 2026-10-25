use crate::{
    Device, RestoreOptions, load_image, tensor, tiling,
    vae::{Decoder, Encoder},
};
use anyhow::{Result, ensure};
use image::imageops::{self, FilterType};
use mlx_gen::weights::Weights;
use mlx_gen_seedvr2::{config::DitConfig, convert, dit::Seedvr2Transformer};
use mlx_rs::{Array, Dtype, ops, random};
use std::path::{Path, PathBuf};

pub const TRANSFORMER: &str = "seedvr2_3b_fp16.safetensors";
pub const VAE: &str = "seedvr2_ema_vae_fp16.safetensors";

pub fn run(
    source: &Path,
    directory: &Path,
    device: Device,
    options: RestoreOptions,
    output: &Path,
) -> Result<Vec<PathBuf>> {
    let RestoreOptions {
        scale,
        seed,
        denoise,
        vae_tile: tile,
        quantize8,
        color_correction,
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
    let dtype = if device == Device::Metal {
        Dtype::Float16
    } else {
        Dtype::Float32
    };
    // Keep the resized RGB reference on the CPU for post-decode color correction.
    let resized = imageops::resize(&input, width, height, FilterType::Lanczos3);
    drop(input);

    // Each model and its transient tensors die inside its evaluated phase.
    let latent = tensor::phase("vae_encode", || {
        let mut pixels = vec![-1f32; ph as usize * pw as usize * 3];
        for y in 0..height {
            for x in 0..width {
                for c in 0..3 {
                    pixels[(y as usize * pw as usize + x as usize) * 3 + c] =
                        resized.get_pixel(x, y)[c] as f32 / 127.5 - 1.;
                }
            }
        }
        let input = Array::from_slice(&pixels, &[1, ph as i32, pw as i32, 3]).as_dtype(dtype)?;
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

    let restored = tensor::phase("dit", || {
        let mut raw = Weights::from_file(directory.join(TRANSFORMER))?;
        // Cast before conversion so the 3B model's shared attention projections
        // remain aliases of one buffer rather than materializing twice.
        raw.cast_all(dtype)?;
        let weights = convert::convert_dit(&raw)?;
        let config = DitConfig::seedvr2_3b();
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
        let mask = Array::ones::<f32>(&[1, 1, 1, nhwc[1], nhwc[2]])?.as_dtype(dtype)?;
        let latent = latent.transpose_axes(&[0, 3, 1, 2])?.expand_dims(2)?;
        let key = random::key(seed)?;
        let noise =
            random::normal::<f32>(latent.shape(), None, None, Some(&key))?.as_dtype(dtype)?;
        let sigma = crate::denoise::sigma(denoise)?;
        let noisy = if sigma == 1.0 {
            noise
        } else {
            ops::add(
                &ops::multiply(&noise, Array::from_f32(sigma))?,
                &ops::multiply(&latent, Array::from_f32(1.0 - sigma))?,
            )?
        };
        let model_input = ops::concatenate_axis(&[&noisy, &latent, &mask], 1)?;
        let prediction =
            transformer.forward(&model_input, &conditioning, &Array::from_f32(sigma * 1000.))?;
        let prediction = if sigma == 1.0 {
            prediction
        } else {
            ops::multiply(&prediction, Array::from_f32(sigma))?
        };
        let restored = ops::subtract(&noisy, prediction)?
            .squeeze_axes(&[2])?
            .transpose_axes(&[0, 2, 3, 1])?
            .as_dtype(dtype)?;
        restored.eval()?;
        Ok(restored)
    })?;
    drop(latent);

    let mut decoded = tensor::phase("vae_decode", || {
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
    // Correct the cropped FP32 output before RGB8 rounding, after models are released.
    tensor::phase("color_correction", || {
        crate::color::apply(&mut decoded, &resized, color_correction)
    })?;
    let result = decoded.rgb8(width, height)?;
    drop(decoded);
    let path = output.join("output.png");
    result.save(&path)?;
    Ok(vec![path])
}
