use crate::database::Store;
use serde_json::Value;
use tauri::State;

#[tauri::command]
pub async fn list_projects(app: tauri::AppHandle) -> Result<Vec<Value>, String> {
    use tauri::Manager;
    tauri::async_runtime::spawn_blocking(move || {
        app.state::<Store>()
            .project_summaries()
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
pub fn write_document(store: &Store, mut document: Value, create: bool) -> Result<(), String> {
    crate::previs::media::strip(&mut document);
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
    tx.commit().map_err(|e| e.to_string())?;
    store.project_changed(id);
    Ok(())
}
#[tauri::command]
pub async fn create_project(store: State<'_, Store>, document: Value) -> Result<(), String> {
    let _guard = store.files.clone().read_owned().await;
    write_document(&store, document, true)
}
#[tauri::command]
pub async fn save_project(
    store: State<'_, Store>,
    mut document: Value,
    mut base: Value,
) -> Result<Value, String> {
    let deleting = document["removedAssetIds"] != base["removedAssetIds"];
    let _read = if deleting {
        None
    } else {
        Some(store.files.clone().read_owned().await)
    };
    let _write = if deleting {
        Some(store.files.clone().write_owned().await)
    } else {
        None
    };
    store.normalize_paths(&mut document);
    store.normalize_paths(&mut base);
    save_merged(&store, document, base).map_err(|e| e.to_string())
}
pub fn save_merged(store: &Store, document: Value, base: Value) -> anyhow::Result<Value> {
    let id = document["id"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Missing project ID"))?;
    anyhow::ensure!(base["id"] == document["id"], "Project identity mismatch");
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let current = crate::project_service::load(&tx, id)?;
    let result = crate::project_service::runtime::execute(
        serde_json::json!({"action":"merge","base":base,"document":document,"current":current}),
    )?;
    anyhow::ensure!(
        result.get("error").is_none(),
        "{}",
        result["error"].as_str().unwrap_or("Project merge failed")
    );
    let document = result["document"].clone();
    crate::project_service::persist(&tx, &current, &document)?;
    let saved = crate::project_service::load(&tx, id)?;
    tx.commit()?;
    store.project_changed(id);
    crate::project_storage::resume_cleanup(&db, &store.media_root())?;
    Ok(saved)
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
