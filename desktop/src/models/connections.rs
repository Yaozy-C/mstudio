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
    .ok_or_else(|| {
        anyhow::Error::from(crate::app_error::AppError::new(
            "MODEL_UNAVAILABLE",
            "connection",
            "Service connection removed",
        ))
    })
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
            "http-json",
            "codex"
        ]
        .contains(&service.kind.as_str()),
        "不支持的连接类型"
    );
    if service.kind == "codex" {
        ensure!(service.endpoint == "codex://local", "Codex 使用本机服务");
        return Ok(());
    }
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
    ensure!(
        service.kind != "codex" || new_key.is_none(),
        "Codex 使用 ChatGPT 登录，无需 API Key"
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
        if service.kind == "codex" {
            ensure!(model.plugin == "codex-image", "Codex 媒体连接仅支持生图");
            model.endpoint = "codex://local/images".into();
            model.has_key = false;
            return Ok(());
        }
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

#[path = "connection_commands.rs"]
mod commands;
#[path = "connection_migration.rs"]
mod migration;
pub use commands::*;
pub use migration::init;
