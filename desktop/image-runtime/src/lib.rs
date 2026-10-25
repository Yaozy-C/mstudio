//! Native model loading and inference. No Python runtime or subprocess.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
mod color;
mod denoise;
#[cfg(target_os = "macos")]
mod depth;
#[cfg(target_os = "macos")]
mod esrgan;
#[cfg(target_os = "macos")]
mod restoration;
#[cfg(all(test, target_os = "macos"))]
mod restoration_tests;
#[cfg(target_os = "macos")]
mod tensor;
#[cfg(target_os = "macos")]
mod tiling;
#[cfg(target_os = "macos")]
mod vae;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Device {
    Cpu,
    Metal,
}

#[derive(Clone, Copy, Debug)]
pub struct RestoreOptions {
    pub scale: u32,
    pub seed: u64,
    pub denoise: f64,
    pub vae_tile: u32,
    pub quantize8: bool,
    pub color_correction: ColorCorrection,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum ColorCorrection {
    #[default]
    Wavelet,
    Lab,
    None,
}

/// Select an explicit shader library before the first MLX call.
pub fn initialize(metallib: &Path) -> Result<()> {
    #[cfg(target_os = "macos")]
    {
        anyhow::ensure!(
            metallib.is_file(),
            "缺少 MLX Metal 算子库：{}",
            metallib.display()
        );
        let path = std::ffi::CString::new(metallib.as_os_str().as_encoded_bytes())?;
        unsafe extern "C" {
            fn mstudio_mlx_shader_path(path: *const std::ffi::c_char);
        }
        // The C++ shim calls MLX's explicit library-path setter; the path is NUL-terminated.
        unsafe {
            mstudio_mlx_shader_path(path.as_ptr());
        }
        // MLX otherwise lets reusable buffers grow to almost all physical memory.
        mlx_rs::memory::set_cache_limit(256 * 1024 * 1024);
    }
    #[cfg(not(target_os = "macos"))]
    let _ = metallib;
    Ok(())
}

#[cfg(target_os = "macos")]
pub fn built_metallib() -> &'static Path {
    Path::new(env!("MSTUDIO_MLX_METALLIB"))
}

#[cfg(target_os = "macos")]
fn load_image(path: &Path) -> Result<image::RgbImage> {
    use image::ImageDecoder;
    let mut decoder = image::ImageReader::open(path)?.into_decoder()?;
    let orientation = decoder.orientation()?;
    let mut image = image::DynamicImage::from_decoder(decoder)?;
    image.apply_orientation(orientation);
    Ok(image.into_rgb8())
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ModelFile {
    pub file: String,
    pub url: String,
    pub sha256: String,
}

pub fn model_files() -> Result<Vec<ModelFile>> {
    Ok(serde_json::from_str(include_str!("../models.json"))?)
}

#[cfg(target_os = "macos")]
fn select_device(device: Device) {
    let selected = match device {
        Device::Cpu => mlx_rs::Device::cpu(),
        Device::Metal => mlx_rs::Device::gpu(),
    };
    mlx_rs::Device::set_default(&selected);
}

pub fn restore(
    source: &Path,
    directory: &Path,
    device: Device,
    scale: u32,
    seed: u64,
    vae_tile: u32,
    output: &Path,
) -> Result<Vec<PathBuf>> {
    restore_with_options(
        source,
        directory,
        device,
        RestoreOptions {
            scale,
            seed,
            denoise: default_denoise(),
            vae_tile,
            quantize8: false,
            color_correction: ColorCorrection::default(),
        },
        output,
    )
}

pub fn restore_with_options(
    source: &Path,
    directory: &Path,
    device: Device,
    options: RestoreOptions,
    output: &Path,
) -> Result<Vec<PathBuf>> {
    anyhow::ensure!(
        [1, 2, 4].contains(&options.scale),
        "Unsupported restoration scale"
    );
    anyhow::ensure!(
        (128..=2048).contains(&options.vae_tile) && options.vae_tile.is_multiple_of(64),
        "Invalid VAE tile size"
    );
    denoise::sigma(options.denoise)?;
    #[cfg(target_os = "macos")]
    {
        if options.denoise == 0.0 {
            use image::ImageDecoder;
            let mut decoder = image::ImageReader::open(source)?.into_decoder()?;
            let orientation = decoder.orientation()?;
            let mut original = image::DynamicImage::from_decoder(decoder)?;
            original.apply_orientation(orientation);
            let original = original.resize_exact(
                original
                    .width()
                    .checked_mul(options.scale)
                    .ok_or_else(|| anyhow::anyhow!("Image width overflow"))?,
                original
                    .height()
                    .checked_mul(options.scale)
                    .ok_or_else(|| anyhow::anyhow!("Image height overflow"))?,
                image::imageops::FilterType::Lanczos3,
            );
            let path = output.join("output.png");
            original.save(&path)?;
            return Ok(vec![path]);
        }
        select_device(device);
        tensor::run_inference(|| restoration::run(source, directory, device, options, output))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (source, directory, device, options, output);
        anyhow::bail!("Native image inference currently requires macOS/MLX")
    }
}

pub const fn default_denoise() -> f64 {
    1.0
}

pub fn depth(
    source: &Path,
    directory: &Path,
    device: Device,
    size: u32,
    output: &Path,
) -> Result<Vec<PathBuf>> {
    #[cfg(target_os = "macos")]
    {
        select_device(device);
        tensor::run_inference(|| depth::run(source, directory, size, output))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (source, directory, device, size, output);
        anyhow::bail!("Native image inference currently requires macOS/MLX")
    }
}

/// Dedicated Real-ESRGAN super resolution. No restoration pass or other models.
pub fn upscale(
    source: &Path,
    directory: &Path,
    device: Device,
    scale: u32,
    tile: u32,
    output: &Path,
) -> Result<Vec<PathBuf>> {
    anyhow::ensure!(
        [2, 4].contains(&scale),
        "Real-ESRGAN 支持 2× 或 4× 超分辨率"
    );
    anyhow::ensure!(
        [128, 256, 512].contains(&tile),
        "ESRGAN 分块须为 128、256 或 512"
    );
    #[cfg(target_os = "macos")]
    {
        select_device(device);
        tensor::run_inference(|| esrgan::run(source, directory, device, scale, tile, output))
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (source, directory, device, scale, tile, output);
        anyhow::bail!("Native image inference currently requires macOS/MLX")
    }
}
