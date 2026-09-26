//! Shared credentials and service addresses. Model IDs and media task endpoints stay on models.
use super::{Model, media};
use crate::database::Store;
use anyhow::{Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ServiceConnection {
    pub id: String,
    pub name: String,
    pub kind: String,
    pub endpoint: String,
    #[serde(default, skip_deserializing)]
    pub has_key: bool,
    #[serde(default, skip_deserializing)]
    pub model_count: usize,
}

pub fn get(db: &Connection, id: &str) -> Result<ServiceConnection> {
    db.query_row(
        "SELECT id,name,kind,endpoint,length(api_key)>0 FROM service_connections WHERE id=?1",
        [id],
        |r| {
            Ok(ServiceConnection {
                id: r.get(0)?,
                name: r.get(1)?,
                kind: r.get(2)?,
                endpoint: r.get(3)?,
                has_key: r.get(4)?,
                model_count: 0,
            })
        },
    )
    .optional()?
    .ok_or_else(|| anyhow::anyhow!("服务连接已移除，请重新选择"))
}
pub fn key(db: &Connection, id: &str) -> Result<String> {
    Ok(db.query_row(
        "SELECT api_key FROM service_connections WHERE id=?1",
        [id],
        |r| r.get(0),
    )?)
}
pub fn legacy_fal_key(db: &Connection) -> Result<String> {
    let keys = db
        .prepare("SELECT DISTINCT api_key FROM service_connections WHERE kind='fal'")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    ensure!(
        keys.len() <= 1,
        "旧任务没有记录服务连接，无法确定使用哪个 fal 账号"
    );
    Ok(keys
        .into_iter()
        .next()
        .unwrap_or(super::setting(db, "fal-key")?.unwrap_or_default()))
}
fn validate(service: &ServiceConnection) -> Result<()> {
    ensure!(
        !service.id.is_empty() && service.id.len() <= 100,
        "连接 ID 无效"
    );
    ensure!(
        !service.name.trim().is_empty() && service.name.chars().count() <= 80,
        "请填写连接名称"
    );
    ensure!(
        [
            "openai-compatible",
            "openai-responses",
            "anthropic-native",
            "gemini-native",
            "fal",
            "http-json"
        ]
        .contains(&service.kind.as_str()),
        "不支持的连接类型"
    );
    let url = reqwest::Url::parse(&service.endpoint)?;
    ensure!(
        url.scheme() == "https"
            || (url.scheme() == "http" && crate::assistant::config::is_local(&service.endpoint)),
        "请使用 HTTPS 或本机 HTTP 地址"
    );
    ensure!(
        url.username().is_empty()
            && url.password().is_none()
            && url.query().is_none()
            && url.fragment().is_none(),
        "服务地址不能含凭据、查询参数或片段"
    );
    if service.kind == "fal" {
        ensure!(
            service.endpoint == "https://queue.fal.run",
            "fal 使用固定的官方服务地址"
        );
    }
    if service.kind == "gemini-native" {
        ensure!(
            url.path().trim_matches('/').is_empty(),
            "Gemini 服务地址不附加 /v1 或 /v1beta"
        );
    }
    Ok(())
}
pub fn save(
    db: &Connection,
    mut service: ServiceConnection,
    new_key: Option<String>,
    clear: bool,
) -> Result<()> {
    service.name = service.name.trim().into();
    service.endpoint = service.endpoint.trim().trim_end_matches('/').into();
    validate(&service)?;
    let new_key = new_key
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty());
    ensure!(
        new_key
            .as_ref()
            .is_none_or(|s| s.len() <= 8192 && !s.contains(['\r', '\n'])),
        "API Key 格式无效"
    );
    let old = get(db, &service.id).ok();
    if let Some(old) = &old {
        ensure!(old.kind == service.kind, "已有连接不能改变协议，请新增连接");
    }
    let changed = old
        .as_ref()
        .is_some_and(|old| old.endpoint != service.endpoint);
    ensure!(
        !changed || !old.as_ref().is_some_and(|s| s.has_key) || clear || new_key.is_some(),
        "更改服务地址后，请重新填写密钥或移除旧密钥"
    );
    if changed && service.kind == "http-json" {
        ensure!(
            !media::read(db)?
                .iter()
                .any(|m| m.connection_id.as_deref() == Some(&service.id)),
            "此 HTTP 连接仍有模型使用，请为新地址添加连接后切换模型"
        );
    }
    let secret = if let Some(key) = new_key {
        key
    } else if clear || changed || old.is_none() {
        String::new()
    } else {
        key(db, &service.id)?
    };
    db.execute("INSERT INTO service_connections(id,name,kind,endpoint,api_key) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET name=excluded.name,endpoint=excluded.endpoint,api_key=excluded.api_key",params![service.id,service.name,service.kind,service.endpoint,secret])?;
    Ok(())
}
pub fn list(db: &Connection) -> Result<Vec<ServiceConnection>> {
    let ids = db
        .prepare("SELECT id FROM service_connections ORDER BY rowid")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let texts = db
        .prepare("SELECT data FROM model_profiles")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let media = media::read(db)?;
    ids.iter()
        .map(|id| {
            let mut service = get(db, id)?;
            service.model_count = texts
                .iter()
                .filter(|raw| {
                    serde_json::from_str::<Model>(raw)
                        .is_ok_and(|m| m.connection_id.as_deref() == Some(id))
                })
                .count()
                + media
                    .iter()
                    .filter(|m| m.connection_id.as_deref() == Some(id))
                    .count();
            Ok(service)
        })
        .collect()
}
pub fn bind_text(db: &Connection, model: &mut Model) -> Result<()> {
    if let Some(id) = &model.connection_id {
        let service = get(db, id)?;
        ensure!(
            !["fal", "http-json"].contains(&service.kind.as_str()),
            "此连接不支持对话模型"
        );
        model.profile.endpoint = service.endpoint;
        model.profile.adapter = service.kind;
        model.has_key = service.has_key;
    }
    Ok(())
}
pub fn bind_media(db: &Connection, model: &mut media::MediaModel) -> Result<()> {
    if let Some(id) = &model.connection_id {
        let service = get(db, id)?;
        ensure!(model.plugin == service.kind, "模型接入方式与服务连接不一致");
        match service.kind.as_str() {
            "gemini-native" => model.endpoint = service.endpoint,
            "fal" => {}
            "http-json" => {
                ensure!(
                    reqwest::Url::parse(&model.endpoint)?.origin()
                        == reqwest::Url::parse(&service.endpoint)?.origin(),
                    "提交接口必须与服务连接同源"
                );
            }
            _ => anyhow::bail!("此连接尚不支持媒体生成"),
        }
        model.has_key = service.has_key;
    }
    Ok(())
}
pub fn media_key(db: &Connection, model: &media::MediaModel) -> Result<String> {
    let mut bound = model.clone();
    bind_media(db, &mut bound)?;
    if let Some(id) = &model.connection_id {
        return key(db, id);
    }
    Ok(super::setting(
        db,
        if model.plugin == "fal" {
            "fal-key".into()
        } else {
            format!("media-key:{}", model.id)
        }
        .as_str(),
    )?
    .unwrap_or_default())
}

/// Idempotent migration: merge only an identical service address, protocol and credential.
/// Different accounts are never guessed to be the same connection.
fn migrate_one(
    db: &Connection,
    name: &str,
    kind: &str,
    endpoint: &str,
    secret: &str,
) -> Result<String> {
    let endpoint = endpoint.trim_end_matches('/');
    let endpoint = if kind == "gemini-native" {
        endpoint.trim_end_matches("/v1beta").trim_end_matches("/v1")
    } else {
        endpoint
    };
    if let Some(id)=db.query_row("SELECT id FROM service_connections WHERE kind=?1 AND endpoint=?2 AND api_key=?3 LIMIT 1",params![kind,endpoint,secret],|r|r.get::<_,String>(0)).optional()? {return Ok(id)}
    let id = format!("service-{}", mstudio::media::id());
    db.execute(
        "INSERT INTO service_connections VALUES(?1,?2,?3,?4,?5)",
        params![id, name, kind, endpoint, secret],
    )?;
    Ok(id)
}
pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS service_connections(id TEXT PRIMARY KEY,name TEXT NOT NULL,kind TEXT NOT NULL,endpoint TEXT NOT NULL,api_key TEXT NOT NULL);")?;
    let tx = db.unchecked_transaction()?;
    let rows = tx
        .prepare("SELECT data FROM model_profiles")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for raw in rows {
        let mut model: Model = serde_json::from_str(&raw)?;
        if model.connection_id.is_some() {
            continue;
        }
        let secret = tx
            .query_row(
                "SELECT api_key FROM model_credentials WHERE id=?1 AND endpoint=?2",
                params![model.id, model.profile.endpoint],
                |r| r.get::<_, String>(0),
            )
            .optional()?
            .unwrap_or_default();
        let id = migrate_one(
            &tx,
            &model.name,
            &model.profile.adapter,
            &model.profile.endpoint,
            &secret,
        )?;
        model.connection_id = Some(id);
        tx.execute(
            "UPDATE model_profiles SET data=?2 WHERE id=?1",
            params![model.id, serde_json::to_string(&model)?],
        )?;
        tx.execute("DELETE FROM model_credentials WHERE id=?1", [&model.id])?;
    }
    let fal_key = super::setting(&tx, "fal-key")?.unwrap_or_default();
    let mut media = media::read(&tx)?;
    let fal_id = if !fal_key.is_empty()
        || media
            .iter()
            .any(|m| m.plugin == "fal" && m.connection_id.is_none())
    {
        Some(migrate_one(
            &tx,
            "fal",
            "fal",
            "https://queue.fal.run",
            &fal_key,
        )?)
    } else {
        None
    };
    for model in &mut media {
        if model.plugin == "codex-image" || model.connection_id.is_some() {
            continue;
        }
        let id = if model.plugin == "fal" {
            fal_id.clone().unwrap()
        } else {
            let secret =
                super::setting(&tx, &format!("media-key:{}", model.id))?.unwrap_or_default();
            let endpoint = if model.plugin == "http-json" {
                reqwest::Url::parse(&model.endpoint)?
                    .origin()
                    .ascii_serialization()
            } else {
                model.endpoint.clone()
            };
            migrate_one(&tx, &model.name, &model.plugin, &endpoint, &secret)?
        };
        model.connection_id = Some(id);
        tx.execute(
            "DELETE FROM settings WHERE key=?1",
            [format!("media-key:{}", model.id)],
        )?;
    }
    if !media.is_empty() {
        tx.execute("INSERT INTO settings VALUES('media-models',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",[serde_json::to_string(&media)?])?;
    }
    tx.execute("DELETE FROM settings WHERE key='fal-key'", [])?;
    // Freeze existing queued jobs to their original connection before a model can switch.
    let jobs = tx
        .prepare("SELECT id,data FROM jobs")?
        .query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    for (id, raw) in jobs {
        let mut job: serde_json::Value = serde_json::from_str(&raw)?;
        if job["connectionId"].is_string() {
            continue;
        }
        if let Some(model) = media.iter().find(|m| {
            Some(m.id.as_str()) == job["mediaModelId"].as_str()
                && job["providerId"] == m.plugin
                && job["endpoint"] == m.endpoint
        }) && let Some(connection) = &model.connection_id
        {
            let service = get(&tx, connection)?;
            job["connectionId"] = serde_json::json!(connection);
            job["connectionEndpoint"] = serde_json::json!(service.endpoint);
            tx.execute(
                "UPDATE jobs SET data=?2 WHERE id=?1",
                params![id, job.to_string()],
            )?;
        }
    }
    tx.commit()?;
    Ok(())
}

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
    save(&tx, connection, key, clear_key).map_err(|e| e.to_string())?;
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
    if ["fal", "http-json"].contains(&service.kind.as_str()) {
        return Err("此连接请从模型库添加，或填写模型端点".into());
    }
    let profile = crate::assistant::config::Profile {
        endpoint: service.endpoint,
        adapter: service.kind.clone(),
        ..Default::default()
    };
    let rows = super::commands::fetch_models(&profile, &secret).await?;
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
