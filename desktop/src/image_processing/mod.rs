mod runner;
mod spec;
pub(crate) mod storage;
#[cfg(test)]
mod tests;

use crate::database::Store;
use anyhow::Result;
use rusqlite::OptionalExtension;
use serde::{Deserialize, Serialize};
use spec::{Operation, Runtime};
use tauri::Manager;

static SLOT: std::sync::LazyLock<std::sync::Arc<tokio::sync::Semaphore>> =
    std::sync::LazyLock::new(|| std::sync::Arc::new(tokio::sync::Semaphore::new(1)));

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    input_asset_id: String,
    #[serde(default)]
    input_version_id: Option<String>,
    #[serde(default)]
    name: String,
    operation: Operation,
}

#[derive(Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Receipt {
    input_asset_id: String,
    assets: Vec<mstudio::model::Asset>,
    versions: Vec<crate::image_versions::ImageVersion>,
}

fn runtime(store: &Store) -> Result<Option<Runtime>> {
    let raw: Option<String> = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT value FROM settings WHERE key='image-processing-runtime'",
            [],
            |r| r.get(0),
        )
        .optional()?;
    raw.map(|raw| serde_json::from_str(&raw).map_err(Into::into))
        .transpose()
}

#[tauri::command]
pub fn image_processing_runtime(store: tauri::State<Store>) -> Result<Option<Runtime>, String> {
    runtime(&store).map_err(|e| e.to_string())
}

#[tauri::command]
pub async fn configure_image_processing(
    store: tauri::State<'_, Store>,
    runtime: Option<Runtime>,
) -> Result<Option<Runtime>, String> {
    let runtime = match runtime {
        Some(runtime) => runtime,
        None => {
            let Some(folder) = rfd::AsyncFileDialog::new()
                .set_title("选择图像处理模型目录")
                .pick_folder()
                .await
            else {
                return Ok(None);
            };
            Runtime {
                directory: folder.path().into(),
                device: if cfg!(target_os = "macos") {
                    spec::Device::Metal
                } else {
                    spec::Device::Cpu
                },
            }
        }
    };
    if !runtime.directory.is_absolute() || !runtime.directory.is_dir() {
        return Err("请选择有效的模型目录".into());
    }
    store.db.lock().unwrap().execute(
        "INSERT INTO settings VALUES('image-processing-runtime',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
        [serde_json::to_string(&runtime).map_err(|e| e.to_string())?]
    ).map_err(|e| e.to_string())?;
    Ok(Some(runtime))
}

#[tauri::command]
pub async fn process_image(
    app: tauri::AppHandle,
    project_id: String,
    request: Request,
) -> Result<Receipt, String> {
    let store = app.state::<Store>();
    let guard = crate::project_storage::working(&store, &project_id)
        .await
        .map_err(|e| e.to_string())?;
    let permit = SLOT
        .clone()
        .acquire_owned()
        .await
        .map_err(|e| e.to_string())?;
    let runtime = runtime(&store).map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let _permit = permit;
        process(
            &app.state::<Store>(),
            &project_id,
            request,
            runtime.as_ref(),
        )
        .inspect(|_| crate::project_service::notify(&app, &project_id))
        .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

fn process(
    store: &Store,
    project: &str,
    request: Request,
    runtime: Option<&Runtime>,
) -> Result<Receipt> {
    request.operation.validate()?;
    let name = request.name.trim();
    let name = if name.is_empty() { "未命名" } else { name };
    let (source, parent_version) = if matches!(request.operation, Operation::Depth { .. }) {
        (
            storage::input(store, project, &request.input_asset_id)?,
            None,
        )
    } else {
        let (asset, version) = crate::image_versions::input(
            store,
            project,
            &request.input_asset_id,
            request.input_version_id.as_deref(),
        )?;
        (std::path::PathBuf::from(asset.path), Some(version))
    };
    let id = mstudio::media::id();
    let work = store.media_root().join("image-processing").join(&id);
    crate::project_storage::track_file(store, project, &work)?;
    let outputs = runner::run(&source, &request.operation, runtime, &work)?;
    let engine = match request.operation {
        Operation::Restore { .. } => "seedvr2-3b",
        Operation::Upscale { scale: 2, .. } => "realesrgan-x2plus",
        Operation::Upscale { .. } => "realesrgan-x4plus",
        Operation::Depth { .. } => "depth-anything-v2-vits",
        Operation::Lut { .. } => "ffmpeg-lut3d-tetrahedral",
    };
    let provenance = serde_json::json!({
        "inputAssetId": request.input_asset_id,
        "inputVersionId": parent_version,
        "operation": request.operation,
        "engine": engine,
        "applicationVersion": env!("CARGO_PKG_VERSION"),
        "runtime": if matches!(request.operation, Operation::Lut { .. }) { None } else { runtime },
    });
    let (assets, versions) = if parent_version.is_some() {
        (
            vec![],
            crate::image_versions::save(
                store,
                project,
                &request.input_asset_id,
                name,
                &outputs,
                &provenance,
            )?,
        )
    } else {
        (
            storage::save(store, project, name, &outputs, Some(&provenance))?,
            vec![],
        )
    };
    Ok(Receipt {
        input_asset_id: request.input_asset_id,
        assets,
        versions,
    })
}
