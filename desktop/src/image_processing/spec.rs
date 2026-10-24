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
        vae_tile: u32,
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
                vae_tile,
            } => {
                ensure!([2, 4].contains(scale), "修复超分支持 2× 或 4×");
                ensure!(*seed <= 9_007_199_254_740_991, "随机种子超出安全整数范围");
                ensure!(
                    (128..=2048).contains(vae_tile) && vae_tile % 64 == 0,
                    "VAE 分块须为 128–2048 的 64 倍数"
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
