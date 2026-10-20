use super::{
    Runtime, Session, Video,
    history_store::{self, ContextInfo, Page, Record},
};
use crate::{assistant::standalone_analysis::Analysis, database::Store};
use serde::Serialize;
use std::sync::Arc;
use tauri::Manager;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Outcome {
    pub result: Analysis,
    pub record_id: Option<String>,
    pub save_error: Option<String>,
}
pub async fn archive(
    app: &tauri::AppHandle,
    session: Arc<Session>,
    result: Analysis,
    context: ContextInfo,
    model_id: String,
    question: String,
    language: String,
) -> Outcome {
    let result_copy = result.clone();
    let owner = app.clone();
    let guard = app.state::<Store>().files.clone().read_owned().await;
    let saved = tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<String> {
        let _guard = guard;
        let store = owner.state::<Store>();
        let root = store.media_root();
        let record = Record {
            id: String::new(),
            created_at: 0,
            context,
            video: session.video.clone(),
            question,
            model_id,
            language,
            result: result_copy,
        };
        let record = history_store::stage(&root, &session, record)?;
        history_store::insert(&root, &*store.conn()?, &record)
    })
    .await;
    match saved {
        Ok(Ok(id)) => Outcome {
            result,
            record_id: Some(id),
            save_error: None,
        },
        failure => Outcome {
            result,
            record_id: None,
            save_error: Some(match failure {
                Ok(Err(e)) => e.to_string(),
                Err(e) => e.to_string(),
                _ => unreachable!(),
            }),
        },
    }
}
#[tauri::command]
pub async fn list_video_analysis_records(
    app: tauri::AppHandle,
    query: String,
    offset: usize,
) -> Result<Page, String> {
    let guard = app.state::<Store>().files.clone().read_owned().await;
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<Page> {
        let _guard = guard;
        let store = app.state::<Store>();
        let db = store.conn()?;
        let root = store.media_root();
        let page = history_store::list(&db, &root, &query, offset)?;
        for item in &page.items {
            if let Some(path) = &item.poster {
                app.asset_protocol_scope().allow_file(path)?;
            }
        }
        Ok(page)
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
#[derive(Serialize)]
pub struct OpenedRecord {
    pub record: Record,
    pub video: Option<Video>,
}
#[tauri::command]
pub async fn open_video_analysis_record(
    app: tauri::AppHandle,
    id: String,
) -> Result<OpenedRecord, String> {
    let guard = app.state::<Store>().files.clone().read_owned().await;
    tauri::async_runtime::spawn_blocking(move || -> anyhow::Result<OpenedRecord> {
        let _guard = guard;
        let store = app.state::<Store>();
        let record = history_store::read(&*store.conn()?, &id)?;
        let source = history_store::media_path(&store.media_root(), &record)?;
        if !source.is_file() {
            return Ok(OpenedRecord {
                record,
                video: None,
            });
        }
        let session_id = uuid::Uuid::new_v4().to_string();
        let directory = app
            .path()
            .app_cache_dir()?
            .join("video-analysis")
            .join(&session_id);
        std::fs::create_dir_all(&directory)?;
        let mut session = Session {
            directory,
            video: record.video.clone(),
        };
        let target = session.directory.join(
            std::path::Path::new(&record.video.path)
                .file_name()
                .unwrap(),
        );
        history_store::link_or_copy(&source, &target)?;
        session.video = Video {
            id: session_id.clone(),
            path: target.to_string_lossy().into(),
            ..record.video.clone()
        };
        app.asset_protocol_scope().allow_file(&target)?;
        let video = Some(session.video.clone());
        app.state::<Runtime>()
            .sessions
            .lock()
            .unwrap()
            .insert(session_id, Arc::new(session));
        Ok(OpenedRecord { record, video })
    })
    .await
    .map_err(|e| e.to_string())?
    .map_err(|e| e.to_string())
}
