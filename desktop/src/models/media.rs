use crate::database::Store;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tauri::State;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaModel {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub connection_id: Option<String>,
    pub kind: String,
    pub plugin: String,
    pub endpoint: String,
    pub params: Value,
    pub enabled: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub http: Option<Value>,
    /// Declared reference inputs and generation controls. See `models::capabilities`.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub capabilities: Option<super::capabilities::Capabilities>,
    #[serde(default)]
    pub has_key: bool,
}
pub fn validate(model: &MediaModel) -> Result<()> {
    ensure!(!model.id.is_empty() && model.id.len() <= 80, "模型 ID 无效");
    ensure!(
        !model.name.trim().is_empty() && model.name.chars().count() <= 80,
        "请填写 80 字以内的模型名称"
    );
    ensure!(
        ["image", "video", "audio"].contains(&model.kind.as_str()),
        "不支持的模型插件或类型"
    );
    crate::model_adapters::provider(&model.plugin)?.validate_endpoint(&model.endpoint)?;
    if model.plugin == "codex-image" {
        ensure!(model.kind == "image", "Codex 仅支持图像输出");
    }
    if model.plugin == "gemini-native" {
        ensure!(model.kind == "image", "Gemini 此适配器仅支持图像输出");
        ensure!(
            model.params["model"].as_str().is_some_and(|s| !s.is_empty()
                && s.bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-._".contains(&c))),
            "请填写有效的 Gemini 模型 ID"
        );
    }
    if model.plugin == "http-json" {
        crate::model_adapters::http_json::validate(
            model
                .http
                .as_ref()
                .ok_or_else(|| anyhow::anyhow!("缺少 HTTP 映射"))?,
        )?;
    }
    ensure!(
        model.params.is_object() && model.params.to_string().len() <= 32000,
        "参数须为不超过 32 KB 的 JSON 对象"
    );
    super::capabilities::validate(model)?;
    Ok(())
}
pub(crate) fn read(db: &rusqlite::Connection) -> Result<Vec<MediaModel>> {
    Ok(match super::setting(db, "media-models")? {
        Some(raw) => serde_json::from_str(&raw)?,
        None => Vec::new(),
    })
}
pub(crate) fn resolved(db: &rusqlite::Connection) -> Result<Vec<MediaModel>> {
    let mut models = read(db)?;
    for model in &mut models {
        super::connections::bind_media(db, model)?;
        model.has_key = !super::connections::media_key(db, model)?.is_empty();
    }
    Ok(models)
}
pub(crate) fn write(db: &rusqlite::Connection, models: &[MediaModel]) -> Result<()> {
    db.execute("INSERT INTO settings VALUES('media-models',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_string(models)?])?;
    Ok(())
}
pub fn check_selected(store: &Store, id: &str, endpoint: &str) -> Result<()> {
    let db = store.db.lock().unwrap();
    let models = resolved(&db)?;
    let model = models
        .iter()
        .find(|m| m.id == id)
        .ok_or_else(|| anyhow::anyhow!("模型已移除，请重新选择"))?;
    ensure!(model.enabled, "模型已停用，请重新选择");
    ensure!(
        model.endpoint == endpoint,
        "模型端点已更新，请重新选择后提交"
    );
    validate(model)
}
#[tauri::command]
pub fn media_model_catalog(store: State<Store>) -> Result<Vec<MediaModel>, String> {
    let db = store.db.lock().unwrap();
    resolved(&db).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn save_media_model(
    store: State<Store>,
    mut model: MediaModel,
    key: Option<String>,
    clear_key: Option<bool>,
) -> Result<(), String> {
    model.has_key = false;
    model.name = model.name.trim().into();
    model.endpoint = model.endpoint.trim().into();
    let mut guard = store.db.lock().unwrap();
    let db = guard.transaction().map_err(|e| e.to_string())?;
    super::connections::bind_media(&db, &mut model).map_err(|e| e.to_string())?;
    if model.connection_id.is_some()
        && (key.as_ref().is_some_and(|k| !k.is_empty()) || clear_key.unwrap_or(false))
    {
        return Err("请在服务连接中修改共用密钥".into());
    }
    validate(&model).map_err(|e| e.to_string())?;
    let mut models = read(&db).map_err(|e| e.to_string())?;
    let credential = format!("media-key:{}", model.id);
    let changed = models
        .iter()
        .find(|m| m.id == model.id)
        .is_some_and(|m| m.endpoint != model.endpoint || m.plugin != model.plugin);
    if let Some(key) = key.as_ref().filter(|k| !k.is_empty()) {
        if key.len() > 4096 || key.contains(['\r', '\n']) {
            return Err("密钥格式无效".into());
        }
        db.execute("INSERT INTO settings VALUES(?1,?2) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [&credential, key]).map_err(|e| e.to_string())?;
    } else if clear_key.unwrap_or(false) || changed {
        db.execute("DELETE FROM settings WHERE key=?1", [&credential])
            .map_err(|e| e.to_string())?;
    }
    if let Some(old) = models.iter_mut().find(|p| p.id == model.id) {
        *old = model;
    } else {
        if models.len() >= 100 {
            return Err("最多保存 100 个媒体模型".into());
        }
        models.push(model);
    }
    write(&db, &models).map_err(|e| e.to_string())?;
    db.commit().map_err(|e| e.to_string())
}
#[tauri::command]
pub fn remove_media_model(store: State<Store>, id: String) -> Result<(), String> {
    let mut guard = store.db.lock().unwrap();
    let db = guard.transaction().map_err(|e| e.to_string())?;
    let mut models = read(&db).map_err(|e| e.to_string())?;
    models.retain(|p| p.id != id);
    db.execute(
        "DELETE FROM settings WHERE key=?1",
        [format!("media-key:{id}")],
    )
    .map_err(|e| e.to_string())?;
    write(&db, &models).map_err(|e| e.to_string())?;
    db.commit().map_err(|e| e.to_string())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn disabled_removed_or_changed_model_cannot_submit() {
        let root = std::env::temp_dir().join(format!("mstudio-media-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        let mut model = MediaModel {
            id: "one".into(),
            name: "One".into(),
            kind: "image".into(),
            plugin: "fal".into(),
            endpoint: "fal-ai/flux/schnell".into(),
            params: serde_json::json!({}),
            enabled: true,
            http: None,
            capabilities: None,
            connection_id: None,
            has_key: false,
        };
        write(&store.db.lock().unwrap(), &[model.clone()]).unwrap();
        assert!(check_selected(&store, "one", "fal-ai/flux/schnell").is_ok());
        assert!(check_selected(&store, "missing", "fal-ai/flux/schnell").is_err());
        assert!(check_selected(&store, "one", "fal-ai/flux/dev").is_err());
        model.enabled = false;
        write(&store.db.lock().unwrap(), &[model]).unwrap();
        assert!(check_selected(&store, "one", "fal-ai/flux/schnell").is_err());
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn registry_roundtrip_and_validation() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            .unwrap();
        let mut model = MediaModel {
            id: "image".into(),
            name: "Image".into(),
            kind: "image".into(),
            plugin: "fal".into(),
            endpoint: "fal-ai/flux/schnell".into(),
            params: serde_json::json!({"num_images":1}),
            enabled: true,
            http: None,
            capabilities: None,
            connection_id: None,
            has_key: false,
        };
        validate(&model).unwrap();
        write(&db, &[model.clone()]).unwrap();
        assert_eq!(read(&db).unwrap()[0].endpoint, model.endpoint);
        model.endpoint = "openai/gpt-image-2.5/sunburst/edit".into();
        validate(&model).unwrap();
        model.endpoint = "fal-ai/../secret".into();
        assert!(validate(&model).is_err());
        model.endpoint = "https://evil.example/api?key=secret".into();
        assert!(validate(&model).is_err());
        model.endpoint = "fal-ai/flux/schnell".into();
        model.params = serde_json::json!([]);
        assert!(validate(&model).is_err());
    }
}
