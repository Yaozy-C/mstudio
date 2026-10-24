use crate::load_image;
use anyhow::{Result, ensure};
use image::{
    ImageBuffer, Luma,
    imageops::{FilterType, resize},
};
use mlx_gen::weights::Weights;
use mlx_gen_depth::{DepthAnythingConfig, DepthAnythingV2};
use mlx_rs::Array;
use std::path::{Path, PathBuf};

pub const WEIGHTS: &str = "depth_anything_v2_vits.safetensors";

pub fn run(source: &Path, directory: &Path, size: u32, output: &Path) -> Result<Vec<PathBuf>> {
    let image = load_image(source)?;
    let scale = size as f32 / image.width().min(image.height()) as f32;
    let multiple =
        |v: u32| ((v as f32 * scale / 14.).round() as u32 * 14).max(size.div_ceil(14) * 14);
    let (w, h) = (multiple(image.width()), multiple(image.height()));
    let input = resize(&image, w, h, FilterType::CatmullRom);
    let mean = [0.485, 0.456, 0.406];
    let std = [0.229, 0.224, 0.225];
    let pixels: Vec<f32> = input
        .as_raw()
        .iter()
        .enumerate()
        .map(|(i, v)| (*v as f32 / 255. - mean[i % 3]) / std[i % 3])
        .collect();
    let weights = Weights::from_file(directory.join(WEIGHTS))?;
    let model = DepthAnythingV2::from_weights(&weights, DepthAnythingConfig::small())?;
    weights.materialize_accessed()?;
    drop(weights);
    let depth = model.forward(&Array::from_slice(&pixels, &[1, h as i32, w as i32, 3]))?;
    let depth = crate::tensor::host_array(&depth)?;
    let raw = resize_depth(
        depth.as_slice::<f32>(),
        depth.shape()[1] as u32,
        depth.shape()[0] as u32,
        image.width(),
        image.height(),
    );
    let values = &raw;
    let low = values.iter().copied().fold(f32::INFINITY, f32::min);
    let high = values.iter().copied().fold(f32::NEG_INFINITY, f32::max);
    ensure!(
        values.iter().all(|v| v.is_finite()) && high > low,
        "Depth model returned invalid numeric values"
    );
    let normalized = |v: f32| ((v - low) / (high - low)).clamp(0., 1.);
    let depth8: ImageBuffer<Luma<u8>, Vec<u8>> =
        ImageBuffer::from_fn(image.width(), image.height(), |x, y| {
            Luma([(normalized(raw[(y * image.width() + x) as usize]) * 255.).round() as u8])
        });
    let depth16: ImageBuffer<Luma<u16>, Vec<u16>> =
        ImageBuffer::from_fn(image.width(), image.height(), |x, y| {
            Luma([(normalized(raw[(y * image.width() + x) as usize]) * 65535.).round() as u16])
        });
    let outputs = vec![output.join("depth-8.png"), output.join("depth-16.png")];
    depth8.save(&outputs[0])?;
    depth16.save(&outputs[1])?;
    Ok(outputs)
}

// Depth is an unbounded scalar field. Image color resizers clamp float channels to [0,1].
fn resize_depth(values: &[f32], iw: u32, ih: u32, ow: u32, oh: u32) -> Vec<f32> {
    let sample = |x: u32, y: u32| values[(y * iw + x) as usize];
    (0..oh)
        .flat_map(|y| {
            (0..ow).map(move |x| {
                let fx = ((x as f32 + 0.5) * iw as f32 / ow as f32 - 0.5).max(0.);
                let fy = ((y as f32 + 0.5) * ih as f32 / oh as f32 - 0.5).max(0.);
                let x0 = (fx.floor() as u32).min(iw - 1);
                let y0 = (fy.floor() as u32).min(ih - 1);
                let (x1, y1) = ((x0 + 1).min(iw - 1), (y0 + 1).min(ih - 1));
                let (wx, wy) = (fx - x0 as f32, fy - y0 as f32);
                let top = sample(x0, y0) * (1. - wx) + sample(x1, y0) * wx;
                let bottom = sample(x0, y1) * (1. - wx) + sample(x1, y1) * wx;
                top * (1. - wy) + bottom * wy
            })
        })
        .collect()
}

#[cfg(test)]
mod tests {
    #[test]
    fn depth_resize_preserves_unbounded_model_units() {
        let result = super::resize_depth(&[2., 6., 10., 14.], 2, 2, 4, 4);
        assert_eq!(result[0], 2.);
        assert_eq!(result[15], 14.);
        assert_eq!(result[5], 5.);
    }
}
