use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
pub fn image_bytes(url: &str) -> Result<(&str, Vec<u8>)> {
    ensure!(url.len() <= 60_000_000, "图片超过大小限制");
    let (header, data) = url.split_once(',').context("图片数据无效")?;
    let mime = header
        .strip_prefix("data:")
        .and_then(|s| s.strip_suffix(";base64"))
        .context("需要内嵌参考图片")?;
    ensure!(
        ["image/png", "image/jpeg", "image/webp"].contains(&mime),
        "不支持的图片格式"
    );
    let bytes = STANDARD.decode(data).context("图片编码无效")?;
    ensure!(!bytes.is_empty(), "图片为空");
    Ok((mime, bytes))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn inline_images_are_validated_independently_of_the_provider() {
        assert_eq!(
            image_bytes("data:image/png;base64,YQ==").unwrap(),
            ("image/png", vec![b'a'])
        );
        for value in [
            "data:text/html;base64,YQ==",
            "data:image/png;base64,",
            "data:image/png;base64,???",
            "https://example.com/image.png",
        ] {
            assert!(image_bytes(value).is_err());
        }
    }
}
