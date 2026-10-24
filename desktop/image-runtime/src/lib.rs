//! Native model loading and inference. No Python runtime or subprocess.
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[cfg(target_os = "macos")]
mod depth;
#[cfg(target_os = "macos")]
mod restoration;
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
    #[cfg(target_os = "macos")]
    {
        select_device(device);
        let result = restoration::run(source, directory, device, scale, seed, vae_tile, output);
        // Each job owns its models. Release cached buffers after their tensors have dropped.
        mlx_rs::memory::clear_cache();
        result
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (source, directory, device, scale, seed, vae_tile, output);
        anyhow::bail!("Native image inference currently requires macOS/MLX")
    }
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
        let result = depth::run(source, directory, size, output);
        mlx_rs::memory::clear_cache();
        result
    }
    #[cfg(not(target_os = "macos"))]
    {
        let _ = (source, directory, device, size, output);
        anyhow::bail!("Native image inference currently requires macOS/MLX")
    }
}
