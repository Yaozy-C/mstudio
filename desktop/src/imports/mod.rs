#[cfg(test)]
mod tests;
pub mod transfer;
mod video_downloader;
pub mod video_url;
use crate::{database::Store, project_storage};
use anyhow::{Result, ensure};
use mstudio::{media, model::Asset};
use serde::Serialize;
use std::path::{Path, PathBuf};
use tauri::Manager;

pub const EXTENSIONS: &[&str] = &[
    "mp4", "mov", "m4v", "webm", "mkv", "png", "jpg", "jpeg", "webp", "gif", "heic", "bmp", "tiff",
    "wav", "mp3", "m4a", "aac", "flac", "ogg", "txt", "md", "csv", "json", "pdf",
];
#[derive(Default, Serialize)]
pub struct Imported {
    pub assets: Vec<Asset>,
    pub errors: Vec<String>,
}

pub fn validate(name: &str, size: u64) -> Result<String> {
    let ext = Path::new(name)
        .extension()
        .and_then(|s| s.to_str())
        .unwrap_or("")
        .to_lowercase();
    ensure!(
        EXTENSIONS.contains(&ext.as_str()),
        "不支持此文件格式，请添加图片、视频、音频、PDF 或 UTF-8 文本"
    );
    let limit = match ext.as_str() {
        "txt" | "md" | "csv" | "json" => 120_000,
        "pdf" => 12 * 1024 * 1024,
        _ => 1024 * 1024 * 1024,
    };
    ensure!(
        size > 0 && size <= limit,
        "文件为空或超过大小限制（文本 120 KB，PDF 12 MiB，媒体 1 GiB）"
    );
    Ok(ext)
}

// Stage each import separately so invalid media and failed thumbnail generation leave no loose files.
pub(crate) fn import_source_scoped(
    store: &Store,
    project: &str,
    source: &Path,
    name: &str,
    work: &Path,
    attachment_only: bool,
) -> Result<Asset> {
    import_source_to(store, Some(project), source, name, work, attachment_only)
}

fn import_source_to(
    store: &Store,
    project: Option<&str>,
    source: &Path,
    name: &str,
    work: &Path,
    attachment_only: bool,
) -> Result<Asset> {
    validate(name, std::fs::metadata(source)?.len())?;
    let mut asset = media::import(source, &work.join("prepared"))?;
    asset.name = name.into();
    asset.attachment_only = attachment_only;
    let mut moved = vec![];
    let result = (|| -> Result<Asset> {
        for (field, folder) in [
            (&mut asset.path, "assets"),
            (&mut asset.preview, "previews"),
        ] {
            if field.is_empty() {
                continue;
            }
            let old = PathBuf::from(&*field);
            let directory = store.media_root().join(folder);
            std::fs::create_dir_all(&directory)?;
            let target = directory.join(old.file_name().unwrap());
            if std::fs::rename(&old, &target).is_err() {
                std::fs::copy(&old, &target)?;
                std::fs::remove_file(&old)?;
            }
            *field = target.to_string_lossy().into();
            moved.push(target);
        }
        if let Some(project) = project {
            project_storage::save_asset(store, project, &asset)?;
        } else {
            crate::asset_library::save_global(store, &asset)?;
        }
        Ok(asset)
    })();
    if result.is_err() {
        for path in moved {
            let _ = std::fs::remove_file(path);
        }
    }
    result
}

pub fn discard(store: &Store, project: &str, work: &Path) -> Result<()> {
    if work.exists() {
        std::fs::remove_dir_all(work)?;
    }
    store.db.lock().unwrap().execute(
        "DELETE FROM project_files WHERE project_id=?1 AND path=?2",
        rusqlite::params![project, work.to_string_lossy()],
    )?;
    Ok(())
}

#[cfg(test)]
pub fn import_paths(store: &Store, project: &str, paths: Vec<PathBuf>) -> Imported {
    import_paths_scoped(store, project, paths, false)
}

fn import_paths_scoped(
    store: &Store,
    project: &str,
    paths: Vec<PathBuf>,
    attachment_only: bool,
) -> Imported {
    let mut result = Imported::default();
    for path in paths {
        let name = path
            .file_name()
            .unwrap_or_default()
            .to_string_lossy()
            .into_owned();
        let work = store.media_root().join("imports").join(media::id());
        let imported = (|| -> Result<Asset> {
            ensure!(path.is_file(), "请选择文件，不支持直接导入文件夹");
            project_storage::track_file(store, project, &work)?;
            import_source_scoped(store, project, &path, &name, &work, attachment_only)
        })();
        match imported {
            Ok(asset) => result.assets.push(asset),
            Err(error) => result.errors.push(format!("{name}：{error}")),
        }
        if let Err(error) = discard(store, project, &work) {
            result.errors.push(error.to_string());
        }
    }
    result
}

#[tauri::command]
pub async fn import_media(
    app: tauri::AppHandle,
    project_id: String,
    attachment_only: Option<bool>,
) -> Result<Imported, String> {
    let guard = project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    let paths = rfd::AsyncFileDialog::new()
        .add_filter("图片 / 文本 / 视频 / 音频 / PDF", EXTENSIONS)
        .pick_files()
        .await
        .unwrap_or_default()
        .into_iter()
        .map(|f| f.path().to_path_buf())
        .collect();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        import_paths_scoped(
            &app.state::<Store>(),
            &project_id,
            paths,
            attachment_only.unwrap_or(false),
        )
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_global_media(app: tauri::AppHandle) -> Result<Imported, String> {
    let guard = app.state::<Store>().files.clone().read_owned().await;
    if !app.state::<Store>().media_root().is_dir() {
        return Err("存储目录不可用，请检查存储设备或设置".into());
    }
    let paths = rfd::AsyncFileDialog::new()
        .add_filter("图片 / 文本 / 视频 / 音频 / PDF", EXTENSIONS)
        .pick_files()
        .await
        .unwrap_or_default();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let store = app.state::<Store>();
        let mut result = Imported::default();
        for file in paths {
            let path = file.path();
            let name = path.file_name().unwrap_or_default().to_string_lossy();
            let work = store.media_root().join("imports").join(media::id());
            match import_source_to(&store, None, path, &name, &work, false) {
                Ok(asset) => result.assets.push(asset),
                Err(error) => result.errors.push(format!("{name}：{error}")),
            }
            if work.exists()
                && let Err(error) = std::fs::remove_dir_all(&work)
            {
                result.errors.push(error.to_string());
            }
        }
        result
    })
    .await
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn import_clipboard_files(
    app: tauri::AppHandle,
    project_id: String,
    attachment_only: Option<bool>,
) -> Result<Imported, String> {
    let guard = project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    let paths = clipboard_paths();
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        import_paths_scoped(
            &app.state::<Store>(),
            &project_id,
            paths,
            attachment_only.unwrap_or(false),
        )
    })
    .await
    .map_err(|e| e.to_string())
}

#[cfg(target_os = "macos")]
fn clipboard_paths() -> Vec<PathBuf> {
    use objc2_app_kit::{NSPasteboard, NSPasteboardTypeFileURL};
    let Some(items) = NSPasteboard::generalPasteboard().pasteboardItems() else {
        return vec![];
    };
    items
        .iter()
        .filter_map(|item| {
            // This is a native file reference, never a path inferred from copied plain text.
            let value = item.stringForType(unsafe { NSPasteboardTypeFileURL })?;
            reqwest::Url::parse(&value.to_string())
                .ok()?
                .to_file_path()
                .ok()
        })
        .collect()
}
#[cfg(windows)]
fn clipboard_paths() -> Vec<PathBuf> {
    crate::windows_host::clipboard_paths()
}
#[cfg(not(any(windows, target_os = "macos")))]
fn clipboard_paths() -> Vec<PathBuf> {
    vec![]
}

#[cfg(test)]
#[path = "library_tests.rs"]
mod library_tests;
