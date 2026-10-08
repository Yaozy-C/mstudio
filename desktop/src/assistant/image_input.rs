//! Bound analysis images before base64 expansion; originals remain untouched.
use anyhow::{Context, Result, ensure};
use std::path::Path;

// Twelve images plus base64 expansion leave room for text under the 18 MiB cap.
pub const MAX_IMAGE_BYTES: usize = 1024 * 1024;
pub const MAX_SOURCE_BYTES: u64 = 64 * 1024 * 1024;

pub fn read(path: &Path, ext: &str) -> Result<(&'static str, Vec<u8>)> {
    let mime = match ext {
        "jpg" | "jpeg" => "image/jpeg",
        "png" => "image/png",
        "webp" => "image/webp",
        "gif" => "image/gif",
        _ => anyhow::bail!("请将图片转换为 PNG、JPEG、WebP 或 GIF 后引用"),
    };
    if std::fs::metadata(path)?.len() <= MAX_IMAGE_BYTES as u64 {
        return Ok((mime, std::fs::read(path)?));
    }
    // Never silently turn an animated reference into a single frame.
    let animated_webp = if ext == "webp" {
        let bytes = std::fs::read(path)?;
        let mut offset = 12usize;
        let mut animated = false;
        while offset + 8 <= bytes.len() {
            if &bytes[offset..offset + 4] == b"ANIM" {
                animated = true;
                break;
            }
            let size = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into()?) as usize;
            offset += 8 + size + size % 2;
        }
        animated
    } else {
        false
    };
    if ext == "gif" || animated_webp {
        ensure!(
            std::fs::metadata(path)?.len() <= super::media_input::MAX_BYTES,
            "动图素材超过 12 MiB，请压缩后引用"
        );
        return Ok((mime, std::fs::read(path)?));
    }
    for edge in [2048, 1536, 1024, 768, 512] {
        let filter = format!(
            "scale=w='min(iw,{edge})':h='min(ih,{edge})':force_original_aspect_ratio=decrease,format=rgba"
        );
        let output = mstudio::media::command("ffmpeg")
            .args(["-v", "error", "-nostdin", "-threads", "2", "-i"])
            .arg(path)
            .args([
                "-map",
                "0:v:0",
                "-frames:v",
                "1",
                "-an",
                "-vf",
                &filter,
                "-f",
                "image2pipe",
                "-c:v",
                "png",
                "-threads",
                "2",
                "pipe:1",
            ])
            .output()
            .context("无法启动图片压缩，请检查 FFmpeg 是否可用")?;
        ensure!(
            output.status.success() && !output.stdout.is_empty(),
            "无法读取或压缩参考图，请检查图片文件"
        );
        if output.stdout.len() <= MAX_IMAGE_BYTES {
            return Ok(("image/png", output.stdout));
        }
    }
    anyhow::bail!("参考图压缩后仍然过大，请裁剪后重试")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::assistant::{attachment_tests, attachments};
    use base64::{Engine, engine::general_purpose::STANDARD};
    use serde_json::json;

    #[test]
    fn twelve_large_references_fit_without_changing_originals() {
        let (root, store, mut doc) = attachment_tests::fixture();
        let source = root.join("assets/image.jpg");
        let png = root.join("assets/large.png");
        mstudio::media::run(
            mstudio::media::command("ffmpeg")
                .args([
                    "-v",
                    "error",
                    "-f",
                    "lavfi",
                    "-i",
                    "nullsrc=s=1200x1200,geq=random(1)*255:128:128,format=rgba",
                    "-frames:v",
                    "1",
                    "-threads",
                    "2",
                ])
                .arg(&png),
        )
        .unwrap();
        let original = std::fs::read(&png).unwrap();
        assert!(original.len() > MAX_IMAGE_BYTES);
        let mut refs = vec![];
        for index in 0..12 {
            let mut asset: mstudio::model::Asset =
                serde_json::from_value(doc["assets"][0].clone()).unwrap();
            asset.id = format!("large-{index}");
            asset.path = png.to_string_lossy().into();
            crate::project_storage::save_asset(&store, "p", &asset).unwrap();
            doc["assets"].as_array_mut().unwrap().push(json!(asset));
            refs.push(attachments::Reference {
                kind: "asset".into(),
                id: asset.id,
            });
        }
        assert!(12 * original.len().div_ceil(3) * 4 > 18 * 1024 * 1024);
        let result = attachments::payload(
            &store,
            &doc,
            "检查这些图片",
            &refs,
            &attachment_tests::multimodal(),
        )
        .unwrap();
        assert!(serde_json::to_vec(&result).unwrap().len() < 18 * 1024 * 1024);
        let images: Vec<_> = result
            .as_array()
            .unwrap()
            .iter()
            .filter(|v| v["type"] == "image_url")
            .collect();
        assert_eq!(images.len(), 12);
        for image in images {
            let data = image["image_url"]["url"]
                .as_str()
                .unwrap()
                .strip_prefix("data:image/png;base64,")
                .unwrap();
            let bytes = STANDARD.decode(data).unwrap();
            assert!(bytes.starts_with(b"\x89PNG\r\n\x1a\n"));
            assert!(bytes.len() <= MAX_IMAGE_BYTES);
        }
        assert_eq!(std::fs::read(&png).unwrap(), original);
        assert_eq!(
            read(&source, "jpg").unwrap().1,
            std::fs::read(source).unwrap()
        );
        std::fs::write(&png, vec![0; MAX_IMAGE_BYTES + 1]).unwrap();
        assert!(
            read(&png, "png").is_err(),
            "invalid images must not be silently omitted"
        );
        assert_eq!(
            read(&png, "gif").unwrap().1.len(),
            MAX_IMAGE_BYTES + 1,
            "animations retain their original bytes rather than losing frames"
        );
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
