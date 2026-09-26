mod migration;
#[cfg(test)]
mod tests;

use crate::database::Store;
use anyhow::{Result, ensure};
use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use tauri::{Manager, State};

pub const FOLDERS: &[&str] = &[
    "codex-images",
    "assets",
    "previews",
    "proxies",
    "captions",
    "exports",
    "render-work",
    "voice-work",
    "reference-work",
    "downloads",
    "imports",
];

#[derive(Clone, Serialize, Deserialize)]
pub struct Location {
    pub directory: PathBuf,
    #[serde(default)]
    pub previous: Vec<PathBuf>,
}

pub fn load(db: &Connection, default: &Path) -> Result<Location> {
    let raw: Option<String> = db
        .query_row(
            "SELECT value FROM settings WHERE key='file-storage'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    Ok(match raw {
        Some(raw) => serde_json::from_str(&raw)?,
        None => Location {
            directory: default.into(),
            previous: vec![],
        },
    })
}

pub fn rewrite(value: &mut Value, from: &Path, to: &Path) {
    match value {
        Value::String(text) => {
            if let Ok(relative) = Path::new(text).strip_prefix(from)
                && relative
                    .components()
                    .next()
                    .is_some_and(|p| FOLDERS.iter().any(|f| p.as_os_str() == *f))
            {
                *text = to.join(relative).to_string_lossy().into();
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| rewrite(v, from, to)),
        Value::Object(items) => items.values_mut().for_each(|v| rewrite(v, from, to)),
        _ => {}
    }
}

pub fn allow(app: &tauri::AppHandle, root: &Path) -> Result<()> {
    for folder in ["assets", "previews", "proxies", "exports", "captions"] {
        app.asset_protocol_scope()
            .allow_directory(root.join(folder), true)?;
    }
    Ok(())
}

#[tauri::command]
pub fn storage_settings(store: State<Store>) -> Value {
    let directory = store.media_root();
    serde_json::json!({"directory":directory, "available":directory.is_dir()})
}

#[tauri::command]
pub async fn choose_storage_directory() -> Option<String> {
    rfd::AsyncFileDialog::new()
        .set_title("选择文件存储位置（会创建 Mstudio 子目录）")
        .pick_folder()
        .await
        .map(|f| f.path().join("Mstudio").to_string_lossy().into())
}

#[tauri::command]
pub async fn migrate_storage(app: tauri::AppHandle, directory: String) -> Result<Value, String> {
    let guard = app
        .state::<Store>()
        .files
        .clone()
        .try_write_owned()
        .map_err(|_| "正在读写文件，请等待导入、生成或导出结束后重试".to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let store = app.state::<Store>();
        let result =
            migration::migrate(&store, Path::new(&directory), |target| allow(&app, target));
        result.map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

pub fn statuses(store: &Store, ids: &[String]) -> Result<HashMap<String, bool>> {
    ensure!(ids.len() <= 100_000, "素材数量过多");
    let db = store.db.lock().unwrap();
    let mut query = db.prepare("SELECT data FROM assets WHERE id=?1")?;
    let mut result = HashMap::new();
    for id in ids {
        let raw: Option<String> = query.query_row([id], |r| r.get(0)).optional()?;
        if let Some(raw) = raw {
            let asset: Value = serde_json::from_str(&raw)?;
            result.insert(
                id.clone(),
                !asset["path"]
                    .as_str()
                    .is_some_and(|p| Path::new(p).is_file()),
            );
        }
    }
    Ok(result)
}

#[tauri::command]
pub async fn asset_file_status(
    store: State<'_, Store>,
    ids: Vec<String>,
) -> Result<HashMap<String, bool>, String> {
    let _guard = store.files.clone().read_owned().await;
    statuses(&store, &ids).map_err(|e| e.to_string())
}
