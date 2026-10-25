//! Fixed VAE tile traversal and cosine blending, matching the validated processing path.
use anyhow::{Result, ensure};
use mlx_rs::Array;

pub struct HostImage {
    pub height: usize,
    pub width: usize,
    pub channels: usize,
    pub values: Vec<f32>,
}

impl HostImage {
    pub fn rgb8(&self, width: u32, height: u32) -> Result<image::RgbImage> {
        ensure!(
            self.channels == 3 && width as usize <= self.width && height as usize <= self.height,
            "Invalid decoded image crop"
        );
        Ok(image::RgbImage::from_fn(width, height, |x, y| {
            image::Rgb(std::array::from_fn(|c| {
                ((self.values[(y as usize * self.width + x as usize) * 3 + c] + 1.) * 127.5)
                    .round()
                    .clamp(0., 255.) as u8
            }))
        }))
    }
}

pub fn run(
    input: &Array,
    tile: i32,
    overlap: i32,
    scale: f32,
    channels: i32,
    apply: impl FnMut(&Array) -> Result<Array>,
) -> Result<Array> {
    let output = run_host(input, tile, overlap, scale, channels, apply)?;
    let array = Array::from_slice(
        &output.values,
        &[1, output.height as i32, output.width as i32, channels],
    )
    .as_dtype(input.dtype())?;
    array.eval()?;
    Ok(array)
}

/// Keep decoded pixels in the existing CPU blend buffer; do not upload/cast a full frame.
pub fn run_host(
    input: &Array,
    tile: i32,
    overlap: i32,
    scale: f32,
    channels: i32,
    mut apply: impl FnMut(&Array) -> Result<Array>,
) -> Result<HostImage> {
    ensure!(
        tile > overlap && overlap >= 0 && channels > 0 && scale.is_finite() && scale > 0.,
        "Invalid VAE tiling parameters"
    );
    let shape = input.shape();
    let (height, width) = (shape[1], shape[2]);
    let (out_h, out_w) = (
        (height as f32 * scale) as usize,
        (width as f32 * scale) as usize,
    );
    let mut result = vec![0f32; out_h * out_w * channels as usize];
    let mut count = vec![0f32; out_h * out_w];
    let step = (tile - overlap) as usize;
    for y in (0..height).step_by(step) {
        let y1 = (y + tile).min(height);
        if y > 0 && y1 - y <= overlap {
            continue;
        }
        for x in (0..width).step_by(step) {
            let x1 = (x + tile).min(width);
            if x > 0 && x1 - x <= overlap {
                continue;
            }
            let ys = Array::from_slice(&(y..y1).collect::<Vec<_>>(), &[y1 - y]);
            let xs = Array::from_slice(&(x..x1).collect::<Vec<_>>(), &[x1 - x]);
            let patch = input.take_axis(&ys, 1)?.take_axis(&xs, 2)?;
            let output = crate::tensor::host_array(&apply(&patch)?)?;
            let (ph, pw) = (output.shape()[1] as usize, output.shape()[2] as usize);
            ensure!(
                output.shape()[3] == channels,
                "Unexpected VAE output channels"
            );
            let oy = (y as f32 * scale) as usize;
            let ox = (x as f32 * scale) as usize;
            let ov = (overlap as f32 * scale) as usize;
            let wy = axis_weights(ph, ov.min(ph / 2), y > 0, y1 < height);
            let wx = axis_weights(pw, ov.min(pw / 2), x > 0, x1 < width);
            let pixels = output.as_slice::<f32>();
            for (py, &yw) in wy.iter().enumerate() {
                for (px, &xw) in wx.iter().enumerate() {
                    let weight = yw * xw;
                    let dest = (oy + py) * out_w + ox + px;
                    count[dest] += weight;
                    for c in 0..channels as usize {
                        result[dest * channels as usize + c] +=
                            pixels[(py * pw + px) * channels as usize + c] * weight;
                    }
                }
            }
        }
    }
    for (pixel, weight) in result.chunks_exact_mut(channels as usize).zip(count) {
        for v in pixel {
            *v /= weight.max(1e-6);
        }
    }
    Ok(HostImage {
        height: out_h,
        width: out_w,
        channels: channels as usize,
        values: result,
    })
}

fn axis_weights(size: usize, overlap: usize, leading: bool, trailing: bool) -> Vec<f32> {
    let mut weights = vec![1.; size];
    for i in 0..overlap {
        let t = if overlap == 1 {
            0.
        } else {
            i as f32 / (overlap - 1) as f32
        };
        let ramp = 0.5 - 0.5 * (t * std::f32::consts::PI).cos();
        if leading {
            weights[i] = ramp;
        }
        if trailing {
            weights[size - overlap + i] = 1. - ramp;
        }
    }
    weights
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn overlapping_tiles_preserve_pixels_with_short_edges() {
        crate::initialize(crate::built_metallib()).unwrap();
        let h = 80;
        let w = 112;
        let data: Vec<f32> = (0..h * w * 3).map(|i| i as f32 / 1000.).collect();
        let input = Array::from_slice(&data, &[1, h, w, 3]);
        let output = run(&input, 64, 16, 1., 3, |x| Ok(x.clone())).unwrap();
        output.eval().unwrap();
        for (index, (a, b)) in output.as_slice::<f32>().iter().zip(data).enumerate() {
            assert!(
                (a - b).abs() < 1e-4,
                "pixel {index}: actual={a}, expected={b}"
            );
        }
    }

    #[test]
    fn host_output_preserves_float32_detail_and_crops_without_uploading() {
        crate::initialize(crate::built_metallib()).unwrap();
        let input = Array::zeros::<f32>(&[1, 4, 4, 3])
            .unwrap()
            .as_dtype(mlx_rs::Dtype::Float16)
            .unwrap();
        let output = run_host(&input, 4, 0, 1., 3, |_| {
            Ok(Array::from_slice(&[0.123456f32; 48], &[1, 4, 4, 3]))
        })
        .unwrap();
        assert_eq!(output.values, vec![0.123456; 48]);
        let rgb = output.rgb8(3, 2).unwrap();
        assert_eq!(rgb.dimensions(), (3, 2));
        assert!(rgb.pixels().all(|p| p.0 == [143; 3]));
        assert!(output.rgb8(5, 2).is_err());
    }
}
