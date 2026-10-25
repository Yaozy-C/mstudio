use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub enum Operation {
    Depth {
        size: u32,
    },
    Restore {
        scale: u32,
        seed: u64,
        #[serde(default = "mstudio_image_runtime::default_denoise")]
        denoise: f64,
        vae_tile: u32,
        #[serde(default)]
        quantize8: bool,
        #[serde(default)]
        color_correction: mstudio_image_runtime::ColorCorrection,
    },
    Upscale {
        scale: u32,
        tile_size: u32,
    },
    Lut {
        cube: String,
        strength: f32,
    },
}

impl Operation {
    pub fn validate(&self) -> Result<()> {
        match self {
            Self::Depth { size } => ensure!((14..=2048).contains(size), "深度推理尺寸须为 14–2048"),
            Self::Restore {
                scale,
                seed,
                denoise,
                vae_tile,
                ..
            } => {
                ensure!(
                    *scale == 1,
                    "AI 修复保持原尺寸；放大请使用 Real-ESRGAN 超分辨率"
                );
                ensure!(*seed <= 9_007_199_254_740_991, "随机种子超出安全整数范围");
                ensure!(
                    denoise.is_finite() && (0.0..=1.0).contains(denoise),
                    "Denoise 须为 0–1"
                );
                ensure!(
                    (128..=2048).contains(vae_tile) && vae_tile % 64 == 0,
                    "VAE 分块须为 128–2048 的 64 倍数"
                );
            }
            Self::Upscale { scale, tile_size } => {
                ensure!([2, 4].contains(scale), "Real-ESRGAN 支持 2× 或 4× 超分辨率");
                ensure!(
                    [128, 256, 512].contains(tile_size),
                    "ESRGAN 分块须为 128、256 或 512"
                );
            }
            Self::Lut { cube, strength } => {
                ensure!(
                    !cube.trim().is_empty() && cube.len() <= 16_000_000,
                    "LUT 须为不超过 16 MB 的 cube 文本"
                );
                ensure!(
                    strength.is_finite() && (0.0..=1.0).contains(strength),
                    "LUT 强度须为 0–1"
                );
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Runtime {
    pub directory: std::path::PathBuf,
    pub device: Device,
}

pub use mstudio_image_runtime::Device;
