//! Post-decode color matching against the resized input, before RGB8 quantization.
//! Wavelet follows ComfyUI's SeedVR2 color_fix.py (five dilated binomial blurs,
//! replicate padding). LAB uses the pinned runtime's wavelet + LAB transfer,
//! including its reference luminance weight of 0.8.
use crate::{ColorCorrection, tiling::HostImage};
use anyhow::{Result, ensure};
use image::RgbImage;
use mlx_rs::Array;

pub(crate) fn apply(
    decoded: &mut HostImage,
    reference: &RgbImage,
    method: ColorCorrection,
) -> Result<()> {
    if method == ColorCorrection::None {
        return Ok(());
    }
    let (w, h) = (reference.width() as usize, reference.height() as usize);
    ensure!(
        w > 0 && h > 0 && decoded.channels == 3 && w <= decoded.width && h <= decoded.height,
        "Invalid color correction reference dimensions"
    );
    let n = w * h;
    let mut content = vec![0.; n * 3];
    let mut style = vec![0.; n * 3];
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                content[c * n + y * w + x] = decoded.values[(y * decoded.width + x) * 3 + c];
                style[c * n + y * w + x] =
                    reference.get_pixel(x as u32, y as u32)[c] as f32 / 127.5 - 1.;
            }
        }
    }
    let corrected = match method {
        ColorCorrection::Wavelet => {
            for c in 0..3 {
                let range = c * n..(c + 1) * n;
                let (high, _) = decompose(&content[range.clone()], w, h);
                let (_, low) = decompose(&style[range.clone()], w, h);
                for ((out, high), low) in content[range].iter_mut().zip(high).zip(low) {
                    *out = (high + low).clamp(-1., 1.);
                }
            }
            content
        }
        ColorCorrection::Lab => {
            let shape = [1, 3, h as i32, w as i32];
            let corrected = mlx_gen_seedvr2::color::apply_color_correction(
                &Array::from_slice(&content, &shape),
                &Array::from_slice(&style, &shape),
                0.8,
            )?;
            crate::tensor::host_array(&corrected)?
                .as_slice::<f32>()
                .to_vec()
        }
        ColorCorrection::None => unreachable!(),
    };
    ensure!(
        corrected.iter().all(|v| v.is_finite()),
        "Non-finite color correction output"
    );
    for y in 0..h {
        for x in 0..w {
            for c in 0..3 {
                decoded.values[(y * decoded.width + x) * 3 + c] = corrected[c * n + y * w + x];
            }
        }
    }
    Ok(())
}

fn decompose(input: &[f32], w: usize, h: usize) -> (Vec<f32>, Vec<f32>) {
    let mut high = vec![0.; input.len()];
    let mut current = input.to_vec();
    for level in 0..5 {
        let radius = (1 << level).min((w.min(h) / 8).max(1)) as isize;
        let mut low = vec![0.; input.len()];
        for y in 0..h {
            for x in 0..w {
                let mut sum = 0.;
                for (ky, dy) in [-1, 0, 1].into_iter().enumerate() {
                    let sy = (y as isize + dy * radius).clamp(0, h as isize - 1) as usize;
                    for (kx, dx) in [-1, 0, 1].into_iter().enumerate() {
                        let sx = (x as isize + dx * radius).clamp(0, w as isize - 1) as usize;
                        let weight = [1., 2., 1.][ky] * [1., 2., 1.][kx] / 16.;
                        sum += weight * current[sy * w + sx];
                    }
                }
                low[y * w + x] = sum;
            }
        }
        for ((high, current), low) in high.iter_mut().zip(&current).zip(&low) {
            // Match the reference's two in-place operations and their FP32 rounding.
            *high += current;
            *high -= low;
        }
        current = low;
    }
    (high, current)
}

#[cfg(test)]
mod tests;
