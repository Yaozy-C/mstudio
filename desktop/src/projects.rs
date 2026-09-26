use crate::database::Store;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub fn list_projects(store: State<Store>) -> Result<Vec<Value>, String> {
    store.projects().map_err(|e| e.to_string())
}
pub fn write_document(store: &Store, mut document: Value, create: bool) -> Result<(), String> {
    store.normalize_paths(&mut document);
    let id = document["id"].as_str().ok_or("项目缺少 ID")?;
    let name = document["name"].as_str().ok_or("项目缺少名称")?;
    if name.trim().is_empty() {
        return Err("项目名称不能为空".into());
    }
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction().map_err(|e| e.to_string())?;
    if !create {
        let previous: String = tx
            .query_row("SELECT document FROM projects WHERE id=?1", [id], |r| {
                r.get(0)
            })
            .map_err(|_| "项目已删除，不能继续保存".to_string())?;
        let previous: Value = serde_json::from_str(&previous).map_err(|e| e.to_string())?;
        crate::project_storage::remember(&tx, id, &previous).map_err(|e| e.to_string())?;
    }
    let sql = if create {
        "INSERT INTO projects VALUES(?1,?2,?3,unixepoch())"
    } else {
        "UPDATE projects SET name=?2,document=?3,updated=unixepoch() WHERE id=?1"
    };
    let changed = tx
        .execute(sql, rusqlite::params![id, name, document.to_string()])
        .map_err(|e| e.to_string())?;
    if changed == 0 {
        return Err("项目已删除，不能继续保存".into());
    }
    crate::project_storage::remember(&tx, id, &document).map_err(|e| e.to_string())?;
    tx.commit().map_err(|e| e.to_string())
}
#[tauri::command]
pub async fn create_project(store: State<'_, Store>, document: Value) -> Result<(), String> {
    let _guard = store.files.clone().read_owned().await;
    write_document(&store, document, true)
}
#[tauri::command]
pub async fn save_project(store: State<'_, Store>, document: Value) -> Result<(), String> {
    let _guard = store.files.clone().read_owned().await;
    write_document(&store, document, false)
}
#[tauri::command]
pub async fn delete_project(app: tauri::AppHandle, id: String) -> Result<(), String> {
    use tauri::Manager;
    let guard = app.state::<Store>().files.clone().write_owned().await;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        crate::project_storage::remove(&app.state::<Store>(), &id).map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub fn get_settings(store: State<Store>) -> Result<Value, String> {
    let connections =
        crate::models::connections::list(&store.db.lock().unwrap()).map_err(|e| e.to_string())?;
    Ok(serde_json::json!({"falConfigured":connections.iter().any(|s|s.kind=="fal" && s.has_key)}))
}
#[tauri::command]
pub fn save_setting(store: State<Store>, key: String, value: String) -> Result<(), String> {
    if !["fal-key"].contains(&key.as_str()) {
        return Err("未知设置".into());
    }
    let db = store.db.lock().unwrap();
    let mut connections = crate::models::connections::list(&db)
        .map_err(|e| e.to_string())?
        .into_iter()
        .filter(|s| s.kind == "fal")
        .collect::<Vec<_>>();
    if connections.len() > 1 {
        return Err("请在服务连接中选择需要修改的 fal 账号".into());
    }
    let connection = connections
        .pop()
        .unwrap_or(crate::models::connections::ServiceConnection {
            id: format!("service-{}", mstudio::media::id()),
            name: "fal".into(),
            kind: "fal".into(),
            endpoint: "https://queue.fal.run".into(),
            has_key: false,
            model_count: 0,
        });
    crate::models::connections::save(&db, connection, Some(value.clone()), value.is_empty())
        .map_err(|e| e.to_string())
}
