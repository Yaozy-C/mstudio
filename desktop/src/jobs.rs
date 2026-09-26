mod persistence;
use crate::database::Store;
use anyhow::{Context, Result, ensure};
use persistence::credential;
pub use persistence::reserve;
use reqwest::Client;
use serde_json::{Value, json};
use tauri::Manager;

pub fn client() -> Result<Client> {
    Ok(Client::builder()
        .redirect(reqwest::redirect::Policy::none())
        .timeout(std::time::Duration::from_secs(90))
        .build()?)
}
pub fn save(store: &Store, job: &Value) -> Result<()> {
    let db = store.db.lock().unwrap();
    let mut job = job.clone();
    store.normalize_paths(&mut job);
    let written = db.execute(
        "INSERT INTO jobs SELECT ?1,?2,?3 WHERE EXISTS(SELECT 1 FROM projects WHERE id=?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
        rusqlite::params![
            job["id"].as_str(),
            job["projectId"].as_str(),
            job.to_string()
        ],
    )?;
    ensure!(written == 1, "项目已删除");
    Ok(())
}
pub fn get(store: &Store, id: &str) -> Result<Value> {
    let s: String =
        store
            .db
            .lock()
            .unwrap()
            .query_row("SELECT data FROM jobs WHERE id=?1", [id], |r| r.get(0))?;
    Ok(serde_json::from_str(&s)?)
}
#[cfg(test)]
pub use crate::model_adapters::queue_url;
#[tauri::command]
pub fn list_jobs(app: tauri::AppHandle, project_id: String) -> Result<Vec<Value>, String> {
    let store = app.state::<Store>();
    let db = store.db.lock().unwrap();
    let mut stmt = db
        .prepare("SELECT json_remove(data,'$.input','$.result','$.outputs') FROM jobs WHERE project_id=?1 ORDER BY rowid DESC")
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([project_id], |r| r.get::<_, String>(0))
        .map_err(|e| e.to_string())?;
    rows.map(|r| serde_json::from_str(&r.map_err(|e| e.to_string())?).map_err(|e| e.to_string()))
        .collect()
}
#[tauri::command]
#[allow(clippy::too_many_arguments)]
pub async fn submit_job(
    app: tauri::AppHandle,
    project_id: String,
    endpoint: String,
    input: Value,
    shot: Value,
    approved: bool,
    media_model_id: String,
    agent_id: String,
    agent_revision: u64,
) -> Result<Value, String> {
    let store = app.state::<Store>();
    let storage_lease = crate::project_storage::working(&store, &project_id)
        .await
        .map_err(crate::app_error::rejected)?;
    let agent = crate::assistant::profiles::resolve(&store.db.lock().unwrap(), Some(&agent_id))
        .map_err(crate::app_error::rejected)?;
    if !agent.tool_ids.contains(&"media-generation".into()) || agent_revision != agent.revision {
        return Err(crate::app_error::rejected(
            "Agent 的媒体生成工具权限已变更，请重新确认生成",
        ));
    }
    crate::models::media::check_selected(&store, &media_model_id, &endpoint)
        .map_err(crate::app_error::rejected)?;
    let model = crate::models::media::resolved(&store.db.lock().unwrap())
        .map_err(crate::app_error::rejected)?
        .into_iter()
        .find(|m| m.id == media_model_id)
        .ok_or_else(|| crate::app_error::rejected("模型已移除"))?;
    if agent.tool_ids.contains(&"project-frames".into())
        && !agent
            .tool_ids
            .iter()
            .any(|id| id == "project-production" || id == "project-edit")
        && model.kind != "image"
    {
        return Err(crate::app_error::rejected("分镜画手只能生成图片"));
    }
    if shot["canvasGeneration"]["task"]["kind"] != model.kind {
        return Err(crate::app_error::rejected(
            "生成任务缺少制作上下文或媒体类型不一致",
        ));
    }
    if model.kind == "video" {
        let raw: String = store
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT document FROM projects WHERE id=?1",
                [&project_id],
                |row| row.get(0),
            )
            .map_err(crate::app_error::rejected)?;
        let doc: Value = serde_json::from_str(&raw).map_err(crate::app_error::rejected)?;
        crate::canvas_inputs::validate(&doc, &shot).map_err(crate::app_error::rejected)?;
    }
    let mut config = model.http.clone().unwrap_or(json!({}));
    config["kind"] = json!(model.kind);
    submit(
        &app,
        project_id,
        endpoint,
        input,
        shot,
        approved,
        &model.plugin,
        &media_model_id,
        config,
        storage_lease,
    )
    .await
    .map_err(|e| crate::app_error::wire(e, "SUBMISSION_UNKNOWN", "submit"))
}
#[allow(clippy::too_many_arguments)]
async fn submit(
    app: &tauri::AppHandle,
    project_id: String,
    endpoint: String,
    input: Value,
    shot: Value,
    approved: bool,
    provider_id: &str,
    media_model_id: &str,
    mut config: Value,
    storage_lease: tokio::sync::OwnedRwLockReadGuard<()>,
) -> Result<Value> {
    ensure!(approved, "请从生成任务提交请求");
    let provider = crate::model_adapters::provider(provider_id)?;
    provider.validate_endpoint(&endpoint)?;
    ensure!(input.is_object(), "模型参数必须为 JSON 对象");
    let store = app.state::<Store>();
    let (key, connection) = {
        let db = store.db.lock().unwrap();
        let model = crate::models::media::resolved(&db)?
            .into_iter()
            .find(|m| m.id == media_model_id)
            .context("模型已移除")?;
        ensure!(
            model.endpoint == endpoint && model.plugin == provider_id,
            "模型连接已变化，请重新提交"
        );
        let key = crate::models::connections::media_key(&db, &model)?;
        let connection = model
            .connection_id
            .as_ref()
            .map(|id| crate::models::connections::get(&db, id))
            .transpose()?;
        (key, connection)
    };
    ensure!(
        provider_id != "fal" || !key.is_empty(),
        "请先配置 fal API Key"
    );
    let id = shot["submissionId"]
        .as_str()
        .context("缺少提交标识")?
        .to_string();
    ensure!(
        !id.is_empty()
            && id.len() <= 80
            && id.bytes().all(|c| c.is_ascii_alphanumeric() || c == b'-'),
        "提交标识无效"
    );
    if provider_id == "codex-image" {
        let root = store.media_root().join("codex-images").join(&id);
        crate::project_storage::track_file(&store, &project_id, &root)?;
        config["codexRoot"] = json!(root);
    }
    let mut job = json!({"id":id,"projectId":project_id,"endpoint":endpoint,"input":input,"shot":shot,"status":"SUBMITTING","created":std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)?.as_secs()});
    job["providerId"] = json!(provider.id());
    job["mediaModelId"] = json!(media_model_id);
    if let Some(connection) = connection {
        job["connectionId"] = json!(connection.id);
        job["connectionEndpoint"] = json!(connection.endpoint);
    }
    job["providerConfig"] = config.clone();
    if !reserve(&store, &job)? {
        return get(&store, &id);
    }
    let result = provider
        .submit(&key, &endpoint, &input, &config, Some(storage_lease))
        .await;
    match result {
        Ok(update) => {
            update.apply(&mut job)?;
            save(&store, &job)?;
            Ok(job)
        }
        Err(error) => {
            let mut issue =
                crate::app_error::AppError::from_error(&error, "SUBMISSION_UNKNOWN", "submit");
            let rejected = issue.outcome.as_deref() == Some("rejected");
            if !rejected {
                issue.code = "SUBMISSION_UNKNOWN".into();
                issue.outcome = Some("unknown".into());
                issue.retryable = false;
            }
            job["status"] = json!(if rejected { "FAILED" } else { "UNKNOWN" });
            job["error"] = json!(issue.to_string());
            save(&store, &job)?;
            Ok(job)
        }
    }
}
#[tauri::command]
pub async fn refresh_job(app: tauri::AppHandle, id: String) -> Result<Value, String> {
    refresh(&app, &id)
        .await
        .map_err(|e| crate::app_error::wire(e, "JOB_SYNC_FAILED", "status"))
}
async fn refresh(app: &tauri::AppHandle, id: &str) -> Result<Value> {
    let _job_guard = crate::job_locks::acquire(id).await;
    let store = app.state::<Store>();
    let mut job = get(&store, id)?;
    if matches!(
        job["status"].as_str(),
        Some("COMPLETED" | "FAILED" | "CANCELLED")
    ) {
        return Ok(job);
    }
    let provider = crate::model_adapters::for_job(&job)?;
    let key = credential(&store, &job)?;
    let update = provider.refresh(&key, &job).await?;
    let cancelling = job["status"] == "CANCEL_REQUESTED";
    job.as_object_mut().unwrap().remove("error");
    update.apply(&mut job)?;
    if cancelling && matches!(job["status"].as_str(), Some("IN_QUEUE" | "IN_PROGRESS")) {
        job["status"] = json!("CANCEL_REQUESTED");
    }
    save(&store, &job)?;
    Ok(job)
}
#[tauri::command]
pub async fn cancel_job(app: tauri::AppHandle, id: String) -> Result<Value, String> {
    async {
        let _job_guard = crate::job_locks::acquire(&id).await;
        let store = app.state::<Store>();
        let mut job = get(&store, &id)?;
        if matches!(
            job["status"].as_str(),
            Some("COMPLETED" | "FAILED" | "CANCELLED" | "CANCEL_REQUESTED")
        ) {
            return Ok(job);
        }
        let provider = crate::model_adapters::for_job(&job)?;
        let key = credential(&store, &job)?;
        let update = provider.cancel(&key, &job).await?;
        update.apply(&mut job)?;
        save(&store, &job)?;
        Ok::<_, anyhow::Error>(job)
    }
    .await
    .map_err(|e| crate::app_error::wire(e, "JOB_CANCEL_FAILED", "cancel"))
}

#[cfg(test)]
#[path = "jobs/connection_tests.rs"]
mod connection_tests;
