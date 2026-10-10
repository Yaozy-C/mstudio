use super::*;
#[tauri::command]
pub fn service_connections(store: State<Store>) -> Result<Vec<ServiceConnection>, String> {
    list(&store.db.lock().unwrap()).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn save_service_connection(
    store: State<Store>,
    connection: ServiceConnection,
    key: Option<String>,
    clear_key: bool,
) -> Result<(), String> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction().map_err(|e| e.to_string())?;
    save(&tx, connection, key, clear_key)
        .map_err(|e| crate::app_error::wire(e, "VALIDATION_FAILED", "connection_settings"))?;
    tx.commit().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn remove_service_connection(store: State<Store>, id: String) -> Result<(), String> {
    let db = store.db.lock().unwrap();
    let service = list(&db)
        .map_err(|e| e.to_string())?
        .into_iter()
        .find(|s| s.id == id)
        .ok_or("服务连接已移除")?;
    if service.model_count > 0 {
        return Err(format!(
            "还有 {} 个模型使用此连接，请先为它们更换连接或移除模型",
            service.model_count
        ));
    }
    let running:bool=db.query_row("SELECT EXISTS(SELECT 1 FROM jobs WHERE json_extract(data,'$.connectionId')=?1 AND coalesce(json_extract(data,'$.status'),'') NOT IN ('COMPLETED','FAILED','CANCELLED'))",[&id],|r|r.get(0)).map_err(|e|e.to_string())?;
    if running {
        return Err("仍有生成任务使用此连接，请先完成或停止任务".into());
    }
    db.execute("DELETE FROM service_connections WHERE id=?1", [id])
        .map_err(|e| e.to_string())?;
    Ok(())
}
#[tauri::command]
pub async fn discover_service_models(
    store: State<'_, Store>,
    id: String,
) -> Result<Vec<serde_json::Value>, String> {
    let (service, secret) = {
        let db = store.db.lock().unwrap();
        (
            get(&db, &id).map_err(|e| e.to_string())?,
            key(&db, &id).map_err(|e| e.to_string())?,
        )
    };
    if ["dashscope", "fal", "http-json"].contains(&service.kind.as_str()) {
        return Err("此连接请从模型库添加，或填写模型端点".into());
    }
    let profile = crate::assistant::config::Profile {
        endpoint: service.endpoint,
        adapter: service.kind.clone(),
        ..Default::default()
    };
    let rows = super::super::commands::fetch_models(&profile, &secret).await?;
    Ok(rows
        .iter()
        .filter_map(|row| {
            let id = row["id"].as_str().or_else(|| row["name"].as_str())?;
            let id = if service.kind == "gemini-native" {
                id.strip_prefix("models/").unwrap_or(id)
            } else {
                id
            };
            Some(serde_json::json!({"id":id,"name":row["displayName"].as_str().unwrap_or(id)}))
        })
        .collect())
}
