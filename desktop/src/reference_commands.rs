use crate::{database::Store, jobs};
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use mstudio::media;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::Manager;
#[derive(Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Reference {
    asset_id: String,
    start: Option<f64>,
    end: Option<f64>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Uploaded {
    asset_id: String,
    kind: String,
    url: String,
}
fn cdn_url(url: &str) -> Result<String> {
    let u = reqwest::Url::parse(url)?;
    ensure!(
        u.scheme() == "https"
            && u.username().is_empty()
            && u.password().is_none()
            && u.port().is_none()
            && u.host_str()
                .is_some_and(|h| h == "fal.media" || h.ends_with(".fal.media")),
        "参考素材存储地址不受支持"
    );
    Ok(url.into())
}
#[path = "reference_prepare.rs"]
mod preparation;
use preparation::prepare;
#[tauri::command]
pub async fn upload_references(
    app: tauri::AppHandle,
    project_id: String,
    references: Vec<Reference>,
    approved: bool,
    media_model_id: String,
) -> Result<Vec<Uploaded>, String> {
    upload(app, project_id, references, approved, media_model_id)
        .await
        .map_err(|e| crate::app_error::wire(e, "ASSET_IMPORT_FAILED", "reference_prepare"))
}
async fn upload(
    app: tauri::AppHandle,
    project_id: String,
    references: Vec<Reference>,
    approved: bool,
    media_model_id: String,
) -> Result<Vec<Uploaded>> {
    ensure!(approved, "请确认上传本次选定参考");
    ensure!(!references.is_empty(), "请选择参考素材");
    let store = app.state::<Store>();
    let guard = std::sync::Arc::new(crate::project_storage::working(&store, &project_id).await?);
    let model = crate::models::media::resolved(&store.db.lock().unwrap())?
        .into_iter()
        .find(|m| m.id == media_model_id && m.enabled)
        .context("模型不存在或已停用")?;
    let native = ["gemini-native", "codex-image"].contains(&model.plugin.as_str());
    ensure!(
        native || ["fal", "dashscope"].contains(&model.plugin.as_str()),
        "此服务不支持本地参考素材"
    );
    // Only explicitly configured limits apply.
    let limits = crate::models::capabilities::limits(&model);
    let key = if native {
        String::new()
    } else {
        crate::models::connections::media_key(&store.db.lock().unwrap(), &model)?
    };
    ensure!(native || !key.is_empty(), "请先在服务连接配置 API Key");
    let doc: String = store.db.lock().unwrap().query_row(
        "SELECT document FROM projects WHERE id=?1",
        [&project_id],
        |r| r.get(0),
    )?;
    let document: Value = serde_json::from_str(&doc)?;
    let owned = document["assets"].as_array().context("项目素材无效")?;
    let all = store.assets()?;
    let mut videos = 0;
    let mut seconds = 0.;
    let mut planned = Vec::new();
    for r in references {
        ensure!(
            owned.iter().any(|a| a["id"] == r.asset_id),
            "参考素材不属于当前项目"
        );
        let asset = all
            .iter()
            .find(|a| a.id == r.asset_id)
            .context("参考素材丢失")?
            .clone();
        match asset.kind.as_str() {
            "image" => {}
            "video" => {
                videos += 1;
                let start = r.start.unwrap_or(0.);
                let end = r.end.unwrap_or(asset.duration);
                ensure!(
                    start.is_finite()
                        && end.is_finite()
                        && start >= 0.
                        && end <= asset.duration
                        && end > start,
                    "参考区间须有效且不能超出原视频"
                );
                seconds += end - start;
            }
            _ => anyhow::bail!("参考仅支持图片与视频"),
        }
        planned.push((asset, r));
    }
    if let Some(limit) = limits.references {
        ensure!(planned.len() <= limit, "此模型最多上传 {limit} 个参考素材");
    }
    if let Some(limit) = limits.seconds {
        ensure!(seconds <= limit, "参考视频总时长不超过 {limit} 秒");
    }
    ensure!(!native || videos == 0, "此原生图片连接只编码图片参考");
    let root = store.media_root().join("reference-work");
    let client = jobs::client()?;
    let mut uploaded = vec![];
    for (asset, r) in planned {
        let work = root.join(media::id());
        crate::project_storage::track_file(&store, &project_id, &work)?;
        let guard = guard.clone();
        let copy = asset.clone();
        let (bytes, mime) = tauri::async_runtime::spawn_blocking(move || {
            let _guard = guard;
            std::fs::create_dir_all(&work)?;
            let result = prepare(&copy, &r, &work);
            let _ = std::fs::remove_dir_all(work);
            result
        })
        .await??;
        if native {
            uploaded.push(Uploaded {
                asset_id: asset.id,
                kind: asset.kind,
                url: format!("data:{mime};base64,{}", STANDARD.encode(bytes)),
            });
            continue;
        }
        if model.plugin == "dashscope" {
            let url = crate::model_adapters::dashscope::upload(
                &key,
                &model.endpoint,
                model.params["model"].as_str().context("请配置模型 ID")?,
                bytes,
                mime,
            )
            .await?;
            uploaded.push(Uploaded {
                asset_id: asset.id,
                kind: asset.kind,
                url,
            });
            continue;
        }
        let name = if mime == "image/png" {
            "reference.png"
        } else {
            "reference.mp4"
        };
        let initiated = client
            .post("https://rest.fal.ai/storage/upload/initiate?storage_type=fal-cdn-v3")
            .header("Authorization", format!("Key {key}"))
            .header(
                "X-Fal-Object-Lifecycle",
                r#"{"expiration_duration_seconds":604800}"#,
            )
            .json(&serde_json::json!({"content_type":mime,"file_name":name}))
            .send()
            .await
            .map_err(|e| {
                crate::app_error::AppError::from_error(
                    &e.into(),
                    "NETWORK_ERROR",
                    "reference_upload_initiate",
                )
            })?;
        if !initiated.status().is_success() {
            return Err(
                crate::app_error::http_error(initiated, "reference_upload_initiate")
                    .await
                    .into(),
            );
        }
        let data: Value = initiated.json().await.map_err(|e| {
            crate::app_error::AppError::from_error(
                &e.into(),
                "ASSET_IMPORT_FAILED",
                "reference_upload_initiate",
            )
        })?;
        let upload_url = cdn_url(data["upload_url"].as_str().context("缺少上传地址")?)?;
        let url = cdn_url(data["file_url"].as_str().context("缺少参考地址")?)?;
        // Never forward the API credential to storage PUT URLs.
        let sent = client
            .put(upload_url)
            .header("Content-Type", mime)
            .body(bytes)
            .send()
            .await
            .map_err(|e| {
                crate::app_error::AppError::from_error(
                    &e.into(),
                    "NETWORK_ERROR",
                    "reference_upload_transfer",
                )
            })?;
        if !sent.status().is_success() {
            return Err(
                crate::app_error::http_error(sent, "reference_upload_transfer")
                    .await
                    .into(),
            );
        }
        uploaded.push(Uploaded {
            asset_id: asset.id,
            kind: asset.kind,
            url,
        });
    }
    Ok(uploaded)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn storage_is_scoped() {
        assert!(cdn_url("https://v3.fal.media/files/a.png").is_ok());
        for value in [
            "http://v3.fal.media/a",
            "https://fal.media.evil.test/a",
            "https://localhost/a",
            "https://key@fal.media/a",
        ] {
            assert!(cdn_url(value).is_err());
        }
    }
}
