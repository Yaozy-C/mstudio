//! A single local Codex account supplies text models and optional image generation.
use super::{Model, connections, media, storage};
use crate::{database::Store, model_adapters::codex_rpc::Rpc};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use std::time::Duration;
use tauri::State;

async fn models(rpc: &mut Rpc) -> Result<Vec<Value>> {
    let mut rows = Vec::new();
    let mut cursor = Value::Null;
    for _ in 0..10 {
        let response = rpc
            .call(
                "model/list",
                json!({"limit":100,"cursor":cursor,"includeHidden":false}),
            )
            .await?;
        for model in response["data"].as_array().into_iter().flatten() {
            if let Some(id) = model["model"].as_str().or_else(|| model["id"].as_str()) {
                rows.push(json!({"id":id,"displayName":model["displayName"].as_str().unwrap_or(id),"isDefault":model["isDefault"] == true}));
            }
        }
        cursor = response["nextCursor"].clone();
        if cursor.is_null() {
            break;
        }
    }
    Ok(rows)
}
pub async fn model_list() -> Result<Vec<Value>> {
    tokio::time::timeout(Duration::from_secs(30), async {
        let mut rpc = Rpc::start_text().await?;
        let result = models(&mut rpc).await;
        rpc.stop().await;
        result
    })
    .await?
}
async fn inspect() -> Result<Value> {
    let mut rpc = match Rpc::start().await {
        Ok(rpc) => rpc,
        Err(_) => Rpc::start_text().await?,
    };
    let result: Result<Value> = async {
        let account = rpc
            .call("account/read", json!({"refreshToken":false}))
            .await?;
        if account["account"]["type"] != "chatgpt" {
            return Ok(json!({"status":"login_required"}));
        }
        let rows = models(&mut rpc).await?;
        ensure!(!rows.is_empty(), "Codex 未返回可用对话模型");
        // Some text-only Codex builds do not implement the image capability endpoint.
        let caps = rpc
            .call("modelProvider/capabilities/read", json!({}))
            .await
            .unwrap_or_default();
        Ok(json!({"status":"ready","models":rows,"images":caps["imageGeneration"] == true}))
    }
    .await;
    rpc.stop().await;
    result
}

#[tauri::command]
pub async fn connect_codex_service(
    store: State<'_, Store>,
    connection: connections::ServiceConnection,
) -> Result<Value, String> {
    if crate::model_adapters::codex_discovery::binaries()
        .await
        .is_empty()
    {
        return Ok(json!({"status":"missing"}));
    }
    let report = tokio::time::timeout(Duration::from_secs(30), inspect())
        .await
        .map_err(|_| "连接 Codex 超时，请重试".to_owned())?
        .map_err(|e| e.to_string())?;
    if report["status"] != "ready" {
        return Ok(report);
    }
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction().map_err(|e| e.to_string())?;
    let service = sync(&tx, connection, &report).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())?;
    Ok(json!({"status":"ready","connection":service}))
}

fn sync(
    db: &rusqlite::Connection,
    mut service: connections::ServiceConnection,
    report: &Value,
) -> Result<connections::ServiceConnection> {
    ensure!(service.kind == "codex", "请选择 Codex 服务");
    // All entries share the same local account: reuse its connection on repeated setup.
    if let Some(old) = connections::list(db)?
        .into_iter()
        .find(|s| s.kind == "codex")
    {
        service = old;
    }
    service.name = "Codex".into();
    service.endpoint = "codex://local".into();
    connections::save(db, service.clone(), None, false)?;
    let rows = report["models"]
        .as_array()
        .ok_or_else(|| anyhow::anyhow!("缺少 Codex 模型列表"))?;
    let selected = rows
        .iter()
        .find(|m| m["isDefault"] == true)
        .or_else(|| rows.first())
        .ok_or_else(|| anyhow::anyhow!("没有可用模型"))?;
    let id = format!("codex-text-{}", service.id);
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM model_profiles WHERE id=?1)",
        [&id],
        |r| r.get(0),
    )?;
    if !exists {
        storage::save(
            db,
            Model {
                id,
                name: format!(
                    "Codex · {}",
                    selected["displayName"].as_str().unwrap_or("Chat")
                ),
                connection_id: Some(service.id.clone()),
                has_key: false,
                profile: crate::assistant::config::Profile {
                    endpoint: service.endpoint.clone(),
                    adapter: "codex".into(),
                    model: selected["id"].as_str().unwrap().into(),
                    inputs: Default::default(),
                    context_window: None,
                },
            },
            None,
            false,
        )?;
    }
    if report["images"] == true {
        let mut items = media::read(db)?;
        let mut found = false;
        for item in &mut items {
            if item.plugin == "codex-image"
                && (item.connection_id.is_none()
                    || item.connection_id.as_deref() == Some(&service.id))
            {
                item.connection_id = Some(service.id.clone());
                found = true;
            }
        }
        if !found {
            ensure!(items.len() < 100, "最多保存 100 个媒体模型");
            items.push(media::MediaModel {
                id: format!("codex-image-{}", service.id),
                name: "Codex 生图".into(),
                connection_id: Some(service.id.clone()),
                kind: "image".into(),
                plugin: "codex-image".into(),
                endpoint: "codex://local/images".into(),
                params: json!({"model":"codex-image","n":1}),
                enabled: true,
                http: None,
                has_key: false,
            });
        }
        media::write(db, &items)?;
    }
    connections::list(db)?
        .into_iter()
        .find(|s| s.id == service.id)
        .ok_or_else(|| anyhow::anyhow!("连接保存失败"))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn local_service_sync_is_idempotent_and_preserves_existing_models() {
        let root = std::env::temp_dir().join(format!("codex-sync-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        {
            let db = store.db.lock().unwrap();
            let service = connections::ServiceConnection {
                id: "local-codex".into(),
                name: "Codex".into(),
                kind: "codex".into(),
                endpoint: "codex://local".into(),
                has_key: false,
                model_count: 0,
            };
            let report = json!({"models":[{"id":"test-model","displayName":"Test","isDefault":true}],"images":true});
            let saved = sync(&db, service.clone(), &report).unwrap();
            assert_eq!(saved.model_count, 2);
            let saved_again = sync(&db, service, &report).unwrap();
            assert_eq!(saved_again.id, saved.id);
            assert_eq!(saved_again.model_count, 2);
            let model = storage::resolve(&db, None, Some("codex-text-local-codex")).unwrap();
            assert!(model.1.is_empty());
            assert_eq!(model.0.profile.adapter, "codex");
            let images = media::resolved(&db).unwrap();
            assert_eq!(images.len(), 1);
            assert_eq!(images[0].connection_id.as_deref(), Some("local-codex"));
            assert!(!images[0].has_key);
        }
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
