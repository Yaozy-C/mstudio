use crate::{media, model::Asset};
use anyhow::{Result, ensure};
use std::path::Path;

pub fn supported(path: &Path) -> bool {
    matches!(
        path.extension()
            .and_then(|e| e.to_str())
            .map(str::to_lowercase)
            .as_deref(),
        Some("txt" | "md" | "csv" | "json" | "pdf")
    )
}
pub fn import(path: &Path, root: &Path) -> Result<Asset> {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();
    let pdf = ext == "pdf";
    let limit = if pdf { 12 * 1024 * 1024 } else { 120_000 };
    ensure!(
        std::fs::metadata(path)?.len() <= limit,
        "文本资料最多 120 KB，PDF 最多 12 MiB"
    );
    let bytes = std::fs::read(path)?;
    if pdf {
        ensure!(bytes.starts_with(b"%PDF-"), "不是有效的 PDF 文件");
    } else {
        ensure!(
            !std::str::from_utf8(&bytes)?.contains('\0'),
            "请导入 UTF-8 文本文件"
        );
    }
    let id = media::id();
    std::fs::create_dir_all(root.join("assets"))?;
    let target = root.join("assets").join(format!("{id}.{ext}"));
    std::fs::write(&target, bytes)?;
    Ok(Asset {
        missing: false,
        generated: false,
        id,
        name: path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into(),
        kind: if pdf { "document" } else { "text" }.into(),
        path: target.to_string_lossy().into(),
        preview: String::new(),
        duration: 0.,
        width: 0,
        height: 0,
        has_audio: false,
    })
}
