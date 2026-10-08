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
    let _job_guard = crate::job_locks::acquire(id).await;
    let store = app.state::<Store>();
    let mut job = jobs::get(&store, id)?;
    let project = job["projectId"]
        .as_str()
        .context("任务缺少项目")?
        .to_owned();
    let guard = std::sync::Arc::new(crate::project_storage::working(&store, &project).await?);
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
        crate::assistant::result_check::record(
            &store,
            &project,
            &job,
            &serde_json::to_value(&asset)?,
        )?;
        return Ok(asset);
    }
    ensure!(
        ["COMPLETED", "FAILED", "CANCELLED"].contains(&job["status"].as_str().unwrap_or("")),
        "任务尚未结束"
    );
    let output = output_at(&job, index)?;
    let (temp, extension) = if output.url.starts_with("data:") {
        let (mime, bytes) = crate::model_adapters::image_data::image_bytes(&output.url)?;
        let extension = match mime {
            "image/png" => "png",
            "image/webp" => "webp",
            _ => "jpg",
        };
        let temp = downloads.join(format!("download-{}.{}", mstudio::media::id(), extension));
        crate::project_storage::track_file(&store, &project, &temp)?;
        std::fs::write(&temp, bytes)?;
        (temp, extension)
    } else {
        let parsed = reqwest::Url::parse(&output.url)?;
        crate::model_adapters::http_json::endpoint(&output.url)?;
        let mut response = jobs::client()?.get(parsed).send().await?;
        ensure!(response.status().is_success(), "媒体下载失败");
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
        crate::project_storage::track_file(&store, &project, &temp)?;
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
        (temp, extension)
    };
    let root = store.media_root();
    let copy = temp.clone();
    let _guard = guard.clone();
    let app_copy = app.clone();
    let owner = project.clone();
    let name = format!("生成结果-{}-{}.{}", id, index + 1, extension);
    let result = tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let mut asset = mstudio::media::import(&copy, &root)?;
        asset.generated = true;
        asset.name = name;
        crate::project_storage::save_asset(&app_copy.state::<Store>(), &owner, &asset)?;
        Ok::<_, anyhow::Error>(asset)
    })
    .await?;
    let _ = std::fs::remove_file(temp);
    let asset = result?;
    let value = serde_json::to_value(&asset)?;
    if !job["assets"].is_array() {
        job["assets"] = serde_json::json!([]);
    }
    {
        let assets = job["assets"].as_array_mut().unwrap();
        assets.resize(assets.len().max(index + 1), serde_json::Value::Null);
        assets[index] = value.clone();
    }
    crate::assistant::result_check::record(&store, &project, &job, &value)?;
    if index == 0 {
        job["asset"] = value;
    }
    jobs::save(&store, &job)?;
    crate::project_service::notify(app, &project);
    Ok(asset)
}

fn output_at(job: &serde_json::Value, index: usize) -> Result<crate::model_adapters::ModelOutput> {
    if let Some(output) = job["outputs"].get(index) {
        ensure!(output["deleted"] != true, "生成结果已删除");
        return Ok(serde_json::from_value(output.clone())?);
    }
    crate::model_adapters::for_job(job)?
        .outputs(&job["result"])
        .into_iter()
        .nth(index)
        .context("未找到可导入的媒体结果；请检查模型输出类型")
}

#[cfg(test)]
mod deletion_tests {
    use super::*;
    #[test]
    fn deleting_one_output_preserves_other_output_indices() {
        let job = serde_json::json!({"outputs":[{"deleted":true},{"kind":"image","url":"https://example.com/keep.png"}],"result":null});
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
}
