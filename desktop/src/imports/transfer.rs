use super::{discard, import_source, validate};
use crate::{database::Store, project_storage};
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    io::{Seek, SeekFrom, Write},
    path::PathBuf,
};
use tauri::{
    Manager,
    ipc::{InvokeBody, Request},
};

#[derive(Deserialize, Serialize)]
struct Transfer {
    project: String,
    name: String,
    size: u64,
    extension: String,
}
fn directory(store: &Store, id: &str) -> Result<PathBuf> {
    ensure!(
        !id.is_empty()
            && id.len() <= 80
            && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-'),
        "导入标识无效"
    );
    Ok(store.media_root().join("imports").join(id))
}
fn read(store: &Store, project: &str, id: &str) -> Result<(PathBuf, Transfer)> {
    let work = directory(store, id)?;
    let transfer: Transfer = serde_json::from_slice(&std::fs::read(work.join("transfer.json"))?)?;
    ensure!(transfer.project == project, "文件不属于当前项目");
    Ok((work, transfer))
}
pub(super) fn begin(store: &Store, project: &str, name: String, size: u64) -> Result<String> {
    let extension = validate(&name, size)?;
    let name = std::path::Path::new(&name)
        .file_name()
        .and_then(|v| v.to_str())
        .filter(|v| !v.is_empty())
        .ok_or_else(|| anyhow::anyhow!("文件名无效"))?
        .to_string();
    let id = mstudio::media::id();
    let work = directory(store, &id)?;
    project_storage::track_file(store, project, &work)?;
    let result = (|| -> Result<()> {
        std::fs::create_dir_all(&work)?;
        std::fs::write(
            work.join("transfer.json"),
            serde_json::to_vec(&Transfer {
                project: project.into(),
                name,
                size,
                extension: extension.clone(),
            })?,
        )?;
        std::fs::File::create(work.join(format!("content.{extension}")))?;
        Ok(())
    })();
    if result.is_err() {
        let _ = discard(store, project, &work);
    }
    result?;
    Ok(id)
}
pub(super) fn append(
    store: &Store,
    project: &str,
    id: &str,
    offset: u64,
    bytes: &[u8],
) -> Result<()> {
    let _transfer = store.imports.lock().unwrap();
    ensure!(
        !bytes.is_empty() && bytes.len() <= 1024 * 1024,
        "导入分片大小无效"
    );
    let (work, transfer) = read(store, project, id)?;
    let mut file = std::fs::OpenOptions::new()
        .write(true)
        .open(work.join(format!("content.{}", transfer.extension)))?;
    ensure!(
        file.metadata()?.len() == offset,
        "导入进度不一致，请重新粘贴或拖入文件"
    );
    ensure!(
        offset + bytes.len() as u64 <= transfer.size,
        "文件超出声明大小"
    );
    file.seek(SeekFrom::End(0))?;
    file.write_all(bytes)?;
    Ok(())
}
pub(super) fn finish(store: &Store, project: &str, id: &str) -> Result<mstudio::model::Asset> {
    let _transfer = store.imports.lock().unwrap();
    let (work, transfer) = read(store, project, id)?;
    let result = (|| {
        let source = work.join(format!("content.{}", transfer.extension));
        ensure!(
            std::fs::metadata(&source)?.len() == transfer.size,
            "文件尚未传输完成"
        );
        import_source(store, project, &source, &transfer.name, &work)
    })();
    // Ownership remains recorded if temporary cleanup fails; do not lose a successfully imported asset.
    if let Err(error) = discard(store, project, &work) {
        eprintln!("清理导入临时文件失败：{error}");
    }
    result
}
pub(super) fn cancel(store: &Store, project: &str, id: &str) -> Result<()> {
    let _transfer = store.imports.lock().unwrap();
    let (work, _) = read(store, project, id)?;
    discard(store, project, &work)
}
#[tauri::command]
pub async fn begin_import(
    app: tauri::AppHandle,
    project_id: String,
    name: String,
    size: u64,
) -> Result<String, String> {
    let guard = project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        begin(&app.state::<Store>(), &project_id, name, size).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn append_import(app: tauri::AppHandle, request: Request<'_>) -> Result<(), String> {
    let header = |key| {
        request
            .headers()
            .get(key)
            .and_then(|v| v.to_str().ok())
            .ok_or("导入参数缺失")
    };
    let project = header("x-project-id")?.to_string();
    let id = header("x-import-id")?.to_string();
    let offset = header("x-import-offset")?
        .parse::<u64>()
        .map_err(|_| "导入进度无效")?;
    let InvokeBody::Raw(bytes) = request.body() else {
        return Err("导入内容无效".into());
    };
    if bytes.len() > 1024 * 1024 {
        return Err("导入分片过大".into());
    }
    let bytes = bytes.clone();
    let guard = project_storage::working(&app.state::<Store>(), &project)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        append(&app.state::<Store>(), &project, &id, offset, &bytes).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn finish_import(
    app: tauri::AppHandle,
    project_id: String,
    id: String,
) -> Result<mstudio::model::Asset, String> {
    let guard = project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        finish(&app.state::<Store>(), &project_id, &id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn cancel_import(
    app: tauri::AppHandle,
    project_id: String,
    id: String,
) -> Result<(), String> {
    let guard = project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let store = app.state::<Store>();
        cancel(&store, &project_id, &id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
