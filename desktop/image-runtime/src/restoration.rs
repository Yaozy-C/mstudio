use crate::{Device, load_image, tiling, vae::Vae};
use anyhow::Result;
use image::{
    RgbImage,
    imageops::{self, FilterType},
};
use mlx_gen::weights::Weights;
use mlx_gen_seedvr2::{config::DitConfig, convert, dit::Seedvr2Transformer};
use mlx_rs::{Array, Dtype, ops, random};
use std::path::{Path, PathBuf};

pub const TRANSFORMER: &str = "seedvr2_distill_6L_1.4B_sharp_fp16_comfyui.safetensors";
pub const VAE: &str = "seedvr2_ema_vae_fp16.safetensors";

pub fn run(
    source: &Path,
    directory: &Path,
    device: Device,
    scale: u32,
    seed: u64,
    tile: u32,
    output: &Path,
) -> Result<Vec<PathBuf>> {
    let input = load_image(source)?;
    let (width, height) = (input.width() * scale, input.height() * scale);
    let resized = imageops::resize(&input, width, height, FilterType::Lanczos3);
    let dtype = if device == Device::Metal {
        Dtype::Float16
    } else {
        Dtype::Float32
    };
    let raw = Weights::from_file(directory.join(TRANSFORMER))?;
    let mut weights = convert::convert_dit(&raw)?;
    weights.cast_all(dtype)?;
    let config = DitConfig {
        num_layers: 6,
        mm_layers: 6,
        ..DitConfig::seedvr2_7b()
    };
    let transformer = Seedvr2Transformer::from_weights(&weights, &config)?;
    let conditioning = weights.require("positive_conditioning")?.expand_dims(0)?;
    weights.materialize_accessed()?;
    drop(weights);
    drop(raw);
    let mut raw = Weights::from_file(directory.join(VAE))?;
    raw.cast_all(dtype)?;
    let vae = Vae::load(&raw)?;
    raw.materialize_accessed()?;
    drop(raw);

    let ph = height.div_ceil(16) * 16;
    let pw = width.div_ceil(16) * 16;
    let mut pixels = vec![-1f32; (ph * pw * 3) as usize];
    for y in 0..height {
        for x in 0..width {
            for c in 0..3 {
                pixels[((y * pw + x) * 3) as usize + c] =
                    resized.get_pixel(x, y)[c] as f32 / 127.5 - 1.;
            }
        }
    }
    let input = Array::from_slice(&pixels, &[1, ph as i32, pw as i32, 3]).as_dtype(dtype)?;
    let latent = tiling::run(&input, tile as i32, (tile / 4) as i32, 0.125, 16, |x| {
        vae.encoder.forward(x)
    })?;
    let nhwc = latent.shape();
    let latent = latent.transpose_axes(&[0, 3, 1, 2])?.expand_dims(2)?;
    let mask = Array::ones::<f32>(&[1, 1, 1, nhwc[1], nhwc[2]])?.as_dtype(dtype)?;
    let key = random::key(seed)?;
    let noise = random::normal::<f32>(latent.shape(), None, None, Some(&key))?.as_dtype(dtype)?;
    let model_input = ops::concatenate_axis(&[&noise, &latent, &mask], 1)?;
    let prediction = transformer.forward(&model_input, &conditioning, &Array::from_f32(1000.))?;
    let restored = ops::subtract(&noise, prediction)?;
    restored.eval()?;
    let restored = restored
        .squeeze_axes(&[2])?
        .transpose_axes(&[0, 2, 3, 1])?
        .as_dtype(dtype)?;
    let decoded = tiling::run(
        &restored,
        (tile / 8) as i32,
        (tile / 32) as i32,
        8.,
        3,
        |x| vae.decoder.forward(x),
    )?
    .as_dtype(Dtype::Float32)?;
    decoded.eval()?;
    let values = decoded.as_slice::<f32>();
    let result = RgbImage::from_fn(width, height, |x, y| {
        image::Rgb(std::array::from_fn(|c| {
            ((values[((y * pw + x) * 3) as usize + c] + 1.) * 127.5)
                .round()
                .clamp(0., 255.) as u8
        }))
    });
    let path = output.join("output.png");
    result.save(&path)?;
    Ok(vec![path])
}
