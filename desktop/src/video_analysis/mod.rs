//! Independent, temporary reference sessions. Never creates a project or library asset.
mod encoding;
pub mod history;
pub(crate) mod history_store;
#[cfg(test)]
mod history_tests;
mod media;
pub(crate) mod progress;
#[cfg(test)]
mod tests;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};
use tauri::Manager;
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct Runtime {
    sessions: Mutex<HashMap<String, Arc<Session>>>,
    active: Mutex<HashMap<String, CancellationToken>>,
}
pub(super) struct Session {
    directory: PathBuf,
    video: Video,
}
impl Drop for Session {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.directory);
    }
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Video {
    pub id: String,
    pub name: String,
    pub path: String,
    pub duration: f64,
    pub width: u64,
    pub height: u64,
    pub has_audio: bool,
}
#[derive(Deserialize)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Source {
    File,
    Url { url: String },
}
struct Operation<'a> {
    runtime: &'a Runtime,
    id: String,
    token: CancellationToken,
}
impl Runtime {
    fn begin(&self, id: &str) -> Result<Operation<'_>> {
        uuid::Uuid::parse_str(id)?;
        let mut active = self.active.lock().unwrap();
        ensure!(!active.contains_key(id), "视频拆解正在进行");
        let token = CancellationToken::new();
        active.insert(id.into(), token.clone());
        Ok(Operation {
            runtime: self,
            id: id.into(),
            token,
        })
    }
}
impl Drop for Operation<'_> {
    fn drop(&mut self) {
        self.runtime.active.lock().unwrap().remove(&self.id);
    }
}

#[tauri::command]
pub async fn prepare_video_analysis(
    app: tauri::AppHandle,
    id: String,
    source: Source,
    on_progress: tauri::ipc::Channel<progress::Progress>,
) -> Result<Option<Video>, String> {
    async fn prepare(
        app: &tauri::AppHandle,
        id: &str,
        source: Source,
        progress: &progress::Reporter,
    ) -> Result<Option<Video>> {
        let runtime = app.state::<Runtime>();
        let operation = runtime.begin(id)?;
        ensure!(
            !runtime.sessions.lock().unwrap().contains_key(id),
            "参考视频已存在"
        );
        let directory = app.path().app_cache_dir()?.join("video-analysis").join(id);
        std::fs::create_dir_all(&directory)?;
        let mut session = Session {
            directory,
            video: Video {
                id: id.into(),
                name: String::new(),
                path: String::new(),
                duration: 0.,
                width: 0,
                height: 0,
                has_audio: false,
            },
        };
        let prepared = tokio::select! {
            _ = operation.token.cancelled() => anyhow::bail!("已停止视频拆解"),
            result = tokio::time::timeout(std::time::Duration::from_secs(20 * 60), media::prepare(app, &session.directory, source, progress)) => result.map_err(|_| anyhow::anyhow!("视频读取超时，请重试"))??,
        };
        let Some(mut video) = prepared else {
            return Ok(None);
        };
        video.id = id.into();
        session.video = video.clone();
        app.asset_protocol_scope().allow_file(&video.path)?;
        let mut sessions = runtime.sessions.lock().unwrap();
        ensure!(!operation.token.is_cancelled(), "已停止视频拆解");
        sessions.insert(id.into(), Arc::new(session));
        Ok(Some(video))
    }
    prepare(&app, &id, source, &progress::Reporter(Some(on_progress)))
        .await
        .map_err(|e| crate::app_error::wire(e, "ASSET_IMPORT_FAILED", "video_analysis"))
}

#[tauri::command]
pub async fn analyze_reference_video(
    app: tauri::AppHandle,
    id: String,
    model_id: String,
    question: String,
    language: String,
    on_progress: tauri::ipc::Channel<progress::Progress>,
    context: history_store::ContextInfo,
) -> Result<history::Outcome, String> {
    let runtime = app.state::<Runtime>();
    let operation = runtime.begin(&id).map_err(|e| e.to_string())?;
    let session = runtime
        .sessions
        .lock()
        .unwrap()
        .get(&id)
        .cloned()
        .ok_or("请先添加视频")?;
    let progress = progress::Reporter(Some(on_progress));
    progress.stage("encoding");
    let result = tokio::select! {
        _ = operation.token.cancelled() => Err(crate::app_error::cancelled()),
        result = async {
            let path = media::model_copy(&session, &operation.token, &progress).await.map_err(|e| e.to_string())?;
            crate::assistant::standalone_analysis::run(&app, &session.video, &path, &model_id, &question, &language, &progress).await
        } => result,
    }?;
    progress.stage("saving");
    // Once a result is complete, finish saving even if the view changes.
    Ok(history::archive(&app, session, result, context, model_id, question, language).await)
}

#[tauri::command]
pub fn cancel_video_analysis(runtime: tauri::State<Runtime>, id: String) {
    if let Some(token) = runtime.active.lock().unwrap().get(&id) {
        token.cancel();
    }
}
#[tauri::command]
pub fn release_video_analysis(runtime: tauri::State<Runtime>, id: String) {
    cancel_video_analysis(runtime.clone(), id.clone());
    runtime.sessions.lock().unwrap().remove(&id);
}
