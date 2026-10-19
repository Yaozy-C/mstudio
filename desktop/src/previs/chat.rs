use super::{agent::PrevisAgent, editor::Editor, files};
use crate::{database::Store, project_storage};
use anyhow::{Context, Result, ensure};
use serde::Deserialize;
use serde_json::Value;
use std::{collections::HashMap, sync::Mutex};
use tauri::{Emitter, Manager, State};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
pub struct Chats(pub Mutex<HashMap<String, CancellationToken>>);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    pub project_id: String,
    pub file_id: String,
    pub expected_revision: u64,
    pub prompt: String,
    pub model_id: String,
    pub request_id: String,
    #[serde(default)]
    pub image_asset_ids: Vec<String>,
}

#[tauri::command]
pub fn cancel_previs_chat(chats: State<Chats>, request_id: String) {
    if let Some(t) = chats.0.lock().unwrap().get(&request_id) {
        t.cancel();
    }
}

#[tauri::command]
pub async fn previs_chat(app: tauri::AppHandle, request: Request) -> Result<Value, String> {
    super::identifier(&request.request_id).map_err(|e| e.to_string())?;
    let token = CancellationToken::new();
    {
        let state = app.state::<Chats>();
        let mut active = state.0.lock().unwrap();
        if active.contains_key(&request.request_id) {
            return Err("请求已在处理中".into());
        }
        active.insert(request.request_id.clone(), token.clone());
    }
    let result = run(&app, &request, &token)
        .await
        .map_err(|e| crate::app_error::wire(e, "PREVIS_CHAT_FAILED", "previs_chat"));
    app.state::<Chats>()
        .0
        .lock()
        .unwrap()
        .remove(&request.request_id);
    result
}

async fn run(app: &tauri::AppHandle, r: &Request, token: &CancellationToken) -> Result<Value> {
    ensure!(!r.prompt.trim().is_empty(), "请填写修改要求");
    let store = app.state::<Store>();
    // Network and tool execution never hold the storage lock.
    let (file, all, model, key, doc) = {
        let db = store.db.lock().unwrap();
        let file = files::read(&db, &r.project_id, &r.file_id)?.context("请先创建预演文件")?;
        ensure!(
            file.revision == r.expected_revision,
            "文件已有新版本，请刷新"
        );
        let all = files::list(&db, &r.project_id)?;
        let (model, key) = crate::models::resolve(&db, None, Some(&r.model_id))?;
        let doc = crate::project_service::load(&db, &r.project_id)?;
        (file, all, model, key, doc)
    };
    let context = super::chat_context::request(&file, &all, &r.prompt);
    let input = crate::assistant::previs_images::message(
        &store,
        &doc,
        &context,
        &r.image_asset_ids,
        &model.profile,
    )?;
    let images = super::messages::images(&store, &doc, &r.image_asset_ids)?;
    {
        let _guard = project_storage::working(&store, &r.project_id).await?;
        let db = store.db.lock().unwrap();
        super::messages::save(
            &db,
            &r.project_id,
            &r.file_id,
            &r.request_id,
            "user",
            &r.prompt,
            &images,
        )?;
    }
    let _ = app.emit(
        "previs-agent-progress",
        serde_json::json!({
            "projectId":r.project_id,"fileId":r.file_id,"requestId":r.request_id,
            "kind":"request/accepted","value":{},
        }),
    );
    let agent = PrevisAgent {
        editor: Mutex::new(Editor::new(&file, &all)),
        app,
        request: r,
        token,
    };
    let message = agent.run(&model, &key, input).await?;
    let _guard = project_storage::working(&store, &r.project_id).await?;
    let db = store.db.lock().unwrap();
    let file = files::read(&db, &r.project_id, &r.file_id)?.context("File was deleted")?;
    super::messages::save(
        &db,
        &r.project_id,
        &r.file_id,
        &r.request_id,
        "assistant",
        &message,
        &serde_json::json!([]),
    )?;
    Ok(serde_json::json!({"file":file,"message":message}))
}
