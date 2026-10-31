use crate::{database::Store, jobs};
use anyhow::{Context, Result, ensure};
use mstudio::model::Asset;
use std::io::Write;
use tauri::Manager;

#[tauri::command]
pub async fn import_job_result(
    app: tauri::AppHandle,
    id: String,
    index: Option<usize>,
) -> Result<Asset, String> {
    download(&app, &id, index.unwrap_or(0))
        .await
        .map_err(|e| crate::app_error::wire(e, "RESULT_IMPORT_FAILED", "import"))
}
pub(crate) async fn download(app: &tauri::AppHandle, id: &str, index: usize) -> Result<Asset> {
    let store = app.state::<Store>();
    let asset = receive(&store, id, index).await?;
    let job = jobs::get(&store, id)?;
    crate::project_service::notify(app, job["projectId"].as_str().context("任务缺少项目")?);
    Ok(asset)
}

/// Same durable import path used by the worker and headless provider integration checks.
pub(crate) async fn receive(store: &Store, id: &str, index: usize) -> Result<Asset> {
    let _job_guard = crate::job_locks::acquire(id).await;
    let mut job = jobs::get(store, id)?;
    let project = job["projectId"]
        .as_str()
        .context("任务缺少项目")?
        .to_owned();
    let guard = std::sync::Arc::new(crate::project_storage::working(store, &project).await?);
    let downloads = store.media_root().join("downloads");
    std::fs::create_dir_all(&downloads)?;
    if let Some(asset) = job["assets"]
        .get(index)
        .filter(|a| a.is_object())
        .or_else(|| if index == 0 { job.get("asset") } else { None })
    {
        ensure!(asset["deleted"] != true, "生成结果已删除");
        let mut asset: Asset = serde_json::from_value(asset.clone())?;
        asset.generated = true;
        asset.missing = !std::path::Path::new(&asset.path).is_file();
        return Ok(asset);
    }
    ensure!(
        ["COMPLETED", "FAILED", "CANCELLED"].contains(&job["status"].as_str().unwrap_or("")),
        "任务尚未结束"
    );
    let output = output_at(&job, index)?;
    let temp = if output.url.starts_with("data:") {
        let (mime, bytes) = crate::model_adapters::image_data::image_bytes(&output.url)?;
        let extension = match mime {
            "image/png" => "png",
            "image/webp" => "webp",
            _ => "jpg",
        };
        let temp = downloads.join(format!("download-{}.{}", mstudio::media::id(), extension));
        crate::project_storage::track_file(store, &project, &temp)?;
        std::fs::write(&temp, bytes)?;
        temp
    } else {
        let parsed = reqwest::Url::parse(&output.url)?;
        crate::model_adapters::http_json::endpoint(&output.url)?;
        let mut response = jobs::client()?.get(parsed).send().await?;
        if !response.status().is_success() {
            return Err(crate::app_error::AppError::http(
                response.status().as_u16(),
                "import",
                "媒体结果下载被服务拒绝",
            )
            .into());
        }
        const MAX: u64 = 1024 * 1024 * 1024;
        ensure!(
            response.content_length().unwrap_or(0) <= MAX,
            "结果文件超过 1 GB"
        );
        let mime = response
            .headers()
            .get("content-type")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("");
        let extension = if mime.starts_with("image/png") {
            "png"
        } else if mime.starts_with("image/") {
            "jpg"
        } else if mime.starts_with("audio/") {
            "mp3"
        } else {
            "mp4"
        };
        let temp = downloads.join(format!("download-{}.{}", mstudio::media::id(), extension));
        crate::project_storage::track_file(store, &project, &temp)?;
        let mut file = std::fs::File::create(&temp)?;
        let mut size = 0;
        let result: Result<()> = async {
            while let Some(chunk) = response.chunk().await? {
                size += chunk.len() as u64;
                ensure!(size <= MAX, "结果文件超过 1 GB");
                file.write_all(&chunk)?;
            }
            Ok(())
        }
        .await;
        drop(file);
        if let Err(e) = result {
            let _ = std::fs::remove_file(temp);
            return Err(e);
        }
        temp
    };
    let root = store.media_root();
    let copy = temp.clone();
    let _guard = guard.clone();
    let name = crate::project_service::production_names::result_name(&*store.conn()?, &job)?;
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let mut asset = mstudio::media::import(&copy, &root)?;
        asset.generated = true;
        asset.name = name;
        Ok::<_, anyhow::Error>(asset)
    })
    .await?;
    let _ = std::fs::remove_file(temp);
    let asset = result?;
    crate::project_storage::save_asset(store, &project, &asset)?;
    let value = serde_json::to_value(&asset)?;
    if !job["assets"].is_array() {
        job["assets"] = serde_json::json!([]);
    }
    {
        let assets = job["assets"].as_array_mut().unwrap();
        assets.resize(assets.len().max(index + 1), serde_json::Value::Null);
        assets[index] = value.clone();
    }
    if index == 0 {
        job["asset"] = value;
    }
    jobs::save(store, &job)?;
    Ok(asset)
}

fn output_at(job: &serde_json::Value, index: usize) -> Result<crate::model_adapters::ModelOutput> {
    let outputs = job["outputs"]
        .as_array()
        .context("生成任务缺少规范化输出列表")?;
    ensure!(
        outputs.len() == jobs::outputs::count(job)?,
        "实际输出数量与输出列表不一致"
    );
    let output = outputs.get(index).context("未找到可导入的媒体结果")?;
    ensure!(output["deleted"] != true, "生成结果已删除");
    Ok(serde_json::from_value(output.clone())?)
}

#[cfg(test)]
mod deletion_tests {
    use super::*;
    #[test]
    fn deleting_one_output_preserves_other_output_indices() {
        let job = serde_json::json!({"status":"COMPLETED","outputCount":2,"outputs":[{"deleted":true},{"kind":"image","url":"https://example.com/keep.png"}],"result":null});
        assert!(
            output_at(&job, 0)
                .unwrap_err()
                .to_string()
                .contains("已删除")
        );
        assert_eq!(
            output_at(&job, 1).unwrap().url,
            "https://example.com/keep.png"
        );
    }
    #[test]
    fn importing_never_recovers_outputs_from_a_provider_result_body() {
        let job = serde_json::json!({"providerId":"gemini-native","result":{"outputs":[{"kind":"image","url":"data:image/png;base64,YQ=="}]}});
        assert!(
            output_at(&job, 0)
                .unwrap_err()
                .to_string()
                .contains("规范化输出列表")
        );
    }
}
