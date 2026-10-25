use super::spec::{Operation, Runtime};
use anyhow::{Context, Result};
use mstudio::media;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

/// Execute exactly one operation. No retries, parameter substitution or quality decisions.
pub fn run(
    source: &Path,
    operation: &Operation,
    runtime: Option<&Runtime>,
    work: &Path,
) -> Result<Vec<PathBuf>> {
    std::fs::create_dir_all(work)?;
    if let Operation::Lut { cube, strength } = operation {
        std::fs::write(work.join("look.cube"), cube)?;
        let filter = format!(
            "format=gbrp,split[a][b];[b]lut3d=file=look.cube:interp=tetrahedral[c];[a][c]blend=all_expr='A*{}+B*{}'",
            1.0 - strength,
            strength
        );
        media::run(
            Command::new(media::binary("ffmpeg"))
                .current_dir(work)
                .args(["-v", "error", "-y", "-i"])
                .arg(source)
                .args([
                    "-filter_complex",
                    &filter,
                    "-frames:v",
                    "1",
                    "-pix_fmt",
                    "rgb24",
                    "output.png",
                ]),
        )?;
        Ok(vec![work.join("output.png")])
    } else {
        let runtime = runtime.context("请先配置本地图像处理运行环境")?;
        match operation {
            Operation::Upscale { scale, tile_size } => mstudio_image_runtime::upscale(
                source,
                &runtime.directory,
                runtime.device,
                *scale,
                *tile_size,
                work,
            ),
            Operation::Depth { size } => mstudio_image_runtime::depth(
                source,
                &runtime.directory,
                runtime.device,
                *size,
                work,
            ),
            Operation::Restore {
                scale,
                seed,
                denoise,
                vae_tile,
                quantize8,
                color_correction,
            } => mstudio_image_runtime::restore_with_options(
                source,
                &runtime.directory,
                runtime.device,
                mstudio_image_runtime::RestoreOptions {
                    scale: *scale,
                    seed: *seed,
                    denoise: *denoise,
                    vae_tile: *vae_tile,
                    quantize8: *quantize8,
                    color_correction: *color_correction,
                },
                work,
            ),
            Operation::Lut { .. } => unreachable!("LUT is executed by FFmpeg above"),
        }
    }
}
