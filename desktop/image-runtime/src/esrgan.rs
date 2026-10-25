//! Real-ESRGAN x2plus/x4plus RRDBNet. Independent of SeedVR2 and its VAE/DiT.
//! Architecture follows BasicSR's rrdbnet_arch.py (Apache-2.0).
use crate::{Device, tensor};
use anyhow::{Result, ensure};
use image::{ImageDecoder, imageops};
use mlx_gen::{nn::upsample_nearest, weights::Weights};
use mlx_rs::{Array, Dtype, ops};
use std::path::{Path, PathBuf};

pub fn model_file(scale: u32) -> Result<&'static str> {
    match scale {
        2 => Ok("RealESRGAN_x2plus.safetensors"),
        4 => Ok("RealESRGAN_x4plus.safetensors"),
        _ => anyhow::bail!("Real-ESRGAN 支持 2× 或 4× 超分辨率"),
    }
}

struct Conv {
    weight: Array,
    bias: Array,
}
impl Conv {
    fn load(weights: &Weights, name: &str) -> Result<Self> {
        Ok(Self {
            weight: weights
                .require(&format!("{name}.weight"))?
                .transpose_axes(&[0, 2, 3, 1])?,
            bias: weights.require(&format!("{name}.bias"))?.clone(),
        })
    }
    fn forward(&self, x: &Array) -> Result<Array> {
        Ok(ops::add(
            ops::conv2d(x, &self.weight, (1, 1), (1, 1), (1, 1), 1)?,
            &self.bias,
        )?)
    }
}
fn leaky(x: &Array) -> Result<Array> {
    Ok(ops::maximum(x, ops::multiply(x, Array::from_f32(0.2))?)?)
}
struct DenseBlock {
    conv: Vec<Conv>,
}
impl DenseBlock {
    fn load(weights: &Weights, name: &str) -> Result<Self> {
        Ok(Self {
            conv: (1..=5)
                .map(|n| Conv::load(weights, &format!("{name}.conv{n}")))
                .collect::<Result<_>>()?,
        })
    }
    fn forward(&self, x: &Array) -> Result<Array> {
        let mut features = vec![x.clone()];
        for conv in &self.conv[..4] {
            let refs: Vec<_> = features.iter().collect();
            features.push(leaky(&conv.forward(&ops::concatenate_axis(&refs, 3)?)?)?);
        }
        let refs: Vec<_> = features.iter().collect();
        let output = self.conv[4].forward(&ops::concatenate_axis(&refs, 3)?)?;
        Ok(ops::add(x, ops::multiply(output, Array::from_f32(0.2))?)?)
    }
}
struct Rrdb {
    blocks: Vec<DenseBlock>,
}
impl Rrdb {
    fn load(weights: &Weights, index: usize) -> Result<Self> {
        Ok(Self {
            blocks: (1..=3)
                .map(|n| DenseBlock::load(weights, &format!("body.{index}.rdb{n}")))
                .collect::<Result<_>>()?,
        })
    }
    fn forward(&self, x: &Array) -> Result<Array> {
        let mut y = x.clone();
        for block in &self.blocks {
            y = block.forward(&y)?;
        }
        Ok(ops::add(x, ops::multiply(y, Array::from_f32(0.2))?)?)
    }
}
struct RrdbNet {
    scale: u32,
    first: Conv,
    blocks: Vec<Rrdb>,
    body: Conv,
    up1: Conv,
    up2: Conv,
    hr: Conv,
    last: Conv,
}
impl RrdbNet {
    fn load(weights: &Weights, scale: u32) -> Result<Self> {
        let first = Conv::load(weights, "conv_first")?;
        ensure!(
            first.weight.shape() == [64, 3, 3, if scale == 2 { 12 } else { 3 }],
            "ESRGAN 权重与所选倍率不符"
        );
        Ok(Self {
            scale,
            first,
            blocks: (0..23)
                .map(|n| Rrdb::load(weights, n))
                .collect::<Result<_>>()?,
            body: Conv::load(weights, "conv_body")?,
            up1: Conv::load(weights, "conv_up1")?,
            up2: Conv::load(weights, "conv_up2")?,
            hr: Conv::load(weights, "conv_hr")?,
            last: Conv::load(weights, "conv_last")?,
        })
    }
    fn forward(&self, pixels: &Array) -> Result<Array> {
        let input = if self.scale == 2 {
            let s = pixels.shape();
            // PyTorch pixel_unshuffle orders channels as (channel, dy, dx).
            pixels
                .reshape(&[s[0], s[1] / 2, 2, s[2] / 2, 2, s[3]])?
                .transpose_axes(&[0, 1, 3, 5, 2, 4])?
                .reshape(&[s[0], s[1] / 2, s[2] / 2, s[3] * 4])?
        } else {
            pixels.clone()
        };
        let head = self.first.forward(&input)?;
        head.eval()?;
        let mut body = head.clone();
        for block in &self.blocks {
            body = block.forward(&body)?;
            // Bound the lazy graph and release each RRDB's dense intermediates.
            body.eval()?;
        }
        let mut x = ops::add(&head, self.body.forward(&body)?)?;
        x = leaky(&self.up1.forward(&upsample_nearest(&x, 2)?)?)?;
        x = leaky(&self.up2.forward(&upsample_nearest(&x, 2)?)?)?;
        self.last.forward(&leaky(&self.hr.forward(&x)?)?)
    }
}

/// A one-pixel reflection used only to make x2 inputs divisible by two.
fn source_index(index: u32, size: u32) -> u32 {
    if index < size {
        index
    } else {
        size.saturating_sub(2)
    }
}

pub fn run(
    source: &Path,
    directory: &Path,
    _device: Device,
    scale: u32,
    tile: u32,
    output: &Path,
) -> Result<Vec<PathBuf>> {
    let mut decoder = image::ImageReader::open(source)?.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = image::DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    let has_alpha = image.color().has_alpha();
    let image = image.to_rgba8();
    let (width, height) = image.dimensions();
    let ow = width
        .checked_mul(scale)
        .ok_or_else(|| anyhow::anyhow!("Image width overflow"))?;
    let oh = height
        .checked_mul(scale)
        .ok_or_else(|| anyhow::anyhow!("Image height overflow"))?;
    let divisor = if scale == 2 { 2 } else { 1 };
    let (pw, ph) = (
        width.div_ceil(divisor) * divisor,
        height.div_ceil(divisor) * divisor,
    );
    let alpha = has_alpha.then(|| {
        let channel = image::GrayImage::from_fn(width, height, |x, y| {
            image::Luma([image.get_pixel(x, y)[3]])
        });
        imageops::resize(&channel, ow, oh, imageops::FilterType::Lanczos3)
    });
    let result = tensor::phase("esrgan", || {
        let mut weights = Weights::from_file(directory.join(model_file(scale)?))?;
        // These small CNNs use FP32 to preserve the original PTH weights.
        weights.cast_all(Dtype::Float32)?;
        let model = RrdbNet::load(&weights, scale)?;
        weights.materialize_accessed()?;
        drop(weights);
        let mut result = image::RgbaImage::new(ow, oh);
        for y in (0..ph).step_by(tile as usize) {
            for x in (0..pw).step_by(tile as usize) {
                let (end_x, end_y) = ((x + tile).min(pw), (y + tile).min(ph));
                // Context is cropped off, never blended into neighboring cores.
                let (left, top) = (x.saturating_sub(32), y.saturating_sub(32));
                let (right, bottom) = ((end_x + 32).min(pw), (end_y + 32).min(ph));
                let mut pixels = Vec::with_capacity(((right - left) * (bottom - top) * 3) as usize);
                for py in top..bottom {
                    for px in left..right {
                        let p = image.get_pixel(source_index(px, width), source_index(py, height));
                        pixels.extend(p.0[..3].iter().map(|v| *v as f32 / 255.));
                    }
                }
                let input = Array::from_slice(
                    &pixels,
                    &[1, (bottom - top) as i32, (right - left) as i32, 3],
                );
                let prediction = tensor::host_array(&model.forward(&input)?)?;
                let values = prediction.as_slice::<f32>();
                ensure!(
                    values.iter().all(|v| v.is_finite()),
                    "ESRGAN 输出包含无效像素"
                );
                let stride = prediction.shape()[2] as usize;
                for py in y * scale..(end_y * scale).min(oh) {
                    for px in x * scale..(end_x * scale).min(ow) {
                        let i = (((py - top * scale) as usize) * stride
                            + (px - left * scale) as usize)
                            * 3;
                        let rgb = std::array::from_fn::<_, 3, _>(|c| {
                            (values[i + c].clamp(0., 1.) * 255.).round() as u8
                        });
                        result.put_pixel(
                            px,
                            py,
                            image::Rgba([
                                rgb[0],
                                rgb[1],
                                rgb[2],
                                alpha.as_ref().map_or(255, |a| a.get_pixel(px, py)[0]),
                            ]),
                        );
                    }
                }
            }
        }
        Ok(result)
    })?;
    let path = output.join("output.png");
    result.save(&path)?;
    Ok(vec![path])
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    #[ignore = "requires installed ESRGAN weights and Metal"]
    fn original_pytorch_rrdbnet_parity() {
        crate::initialize(crate::built_metallib()).unwrap();
        crate::select_device(Device::Metal);
        let directory = PathBuf::from(std::env::var("MSTUDIO_TEST_IMAGE_MODELS").unwrap());
        let reference = Weights::from_file(
            Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("tests/fixtures/esrgan-reference.safetensors"),
        )
        .unwrap();
        tensor::run_inference(|| {
            for scale in [2, 4] {
                let weights = Weights::from_file(directory.join(model_file(scale)?))?;
                let model = RrdbNet::load(&weights, scale)?;
                let actual = tensor::host_array(&model.forward(reference.require("pixels")?)?)?;
                let expected = tensor::host_array(reference.require(&format!("output_{scale}"))?)?;
                ensure!(actual.shape() == expected.shape(), "wrong output geometry");
                let errors: Vec<f32> = actual
                    .as_slice::<f32>()
                    .iter()
                    .zip(expected.as_slice::<f32>())
                    .map(|(a, b)| (a - b).abs())
                    .collect();
                let mae = errors.iter().sum::<f32>() / errors.len() as f32;
                let max = errors.into_iter().fold(0., f32::max);
                eprintln!("RealESRGAN x{scale} vs original PyTorch: MAE={mae}, max={max}");
                ensure!(
                    mae < 1e-4 && max < 1e-3,
                    "ESRGAN differs from original PyTorch"
                );
            }
            Ok(())
        })
        .unwrap();
    }
}
