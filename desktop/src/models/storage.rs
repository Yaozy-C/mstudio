use super::{Catalog, Model, setting};
use crate::assistant::config;
use anyhow::{Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};

fn get(db: &Connection, id: &str) -> Result<Model> {
    let raw: Option<String> = db
        .query_row("SELECT data FROM model_profiles WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .optional()?;
    let mut model: Model =
        serde_json::from_str(&raw.ok_or_else(|| anyhow::anyhow!("模型连接已移除，请重新选择"))?)?;
    super::connections::bind_text(db, &mut model)?;
    model.has_key = !credential(db, &model)?.is_empty();
    Ok(model)
}

fn credential(db: &Connection, model: &Model) -> Result<String> {
    if let Some(id) = &model.connection_id {
        return super::connections::key(db, id);
    }
    Ok(db
        .query_row(
            "SELECT api_key FROM model_credentials WHERE id=?1 AND endpoint=?2",
            params![model.id, model.profile.endpoint],
            |r| r.get(0),
        )
        .optional()?
        .unwrap_or_default())
}

pub fn catalog(db: &Connection, project: Option<&str>) -> Result<Catalog> {
    let ids = db
        .prepare("SELECT id FROM model_profiles ORDER BY rowid")?
        .query_map([], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let profiles = ids
        .iter()
        .map(|id| get(db, id))
        .collect::<Result<Vec<_>>>()?;
    let default_id =
        setting(db, "default-agent-model")?.filter(|id| profiles.iter().any(|p| &p.id == id));
    let selected_id = if let Some(project) = project {
        db.query_row(
            "SELECT model_id FROM agent_model_preferences WHERE project_id=?1",
            [project],
            |r| r.get::<_, Option<String>>(0),
        )
        .optional()?
        .flatten()
    } else {
        None
    };
    Ok(Catalog {
        profiles,
        default_id,
        selected_id,
    })
}

pub fn resolve(
    db: &Connection,
    project: Option<&str>,
    id: Option<&str>,
) -> Result<(Model, String)> {
    let c = catalog(db, project)?;
    let id = id
        .or(c.selected_id.as_deref())
        .or(c.default_id.as_deref())
        .ok_or_else(|| anyhow::anyhow!("请先在模型中心添加并选择 Agent 模型"))?;
    let model = get(db, id)?;
    config::validate(&model.profile)?;
    let key = credential(db, &model)?;
    ensure!(
        model.profile.adapter == "codex"
            || !key.is_empty()
            || config::is_local(&model.profile.endpoint),
        "该模型尚未配置 API Key，请到模型中心补充"
    );
    Ok((model, key))
}

pub fn save(db: &Connection, mut model: Model, key: Option<String>, clear: bool) -> Result<()> {
    super::connections::bind_text(db, &mut model)?;
    ensure!(
        model.connection_id.is_none() || (key.as_ref().is_none_or(|k| k.is_empty()) && !clear),
        "请在服务连接中修改共用密钥"
    );
    model.name = model.name.trim().into();
    model.profile.endpoint = model.profile.endpoint.trim().trim_end_matches('/').into();
    model.profile.model = model.profile.model.trim().into();
    ensure!(
        !model.id.is_empty() && model.id.len() <= 80,
        "模型连接 ID 无效"
    );
    ensure!(
        !model.name.is_empty() && model.name.chars().count() <= 80,
        "请填写 80 字以内的连接名称"
    );
    config::validate(&model.profile)?;
    let old = db
        .query_row(
            "SELECT data FROM model_profiles WHERE id=?1",
            [&model.id],
            |r| r.get::<_, String>(0),
        )
        .optional()?
        .map(|s| serde_json::from_str::<Model>(&s))
        .transpose()?;
    let key = key.map(|s| s.trim().to_string()).filter(|s| !s.is_empty());
    ensure!(
        key.as_ref()
            .is_none_or(|k| k.len() <= 8192 && !k.contains(['\n', '\r'])),
        "API Key 格式无效"
    );
    let changed = old
        .as_ref()
        .is_some_and(|p| p.profile.endpoint != model.profile.endpoint);
    let old_key = old
        .as_ref()
        .map(|p| credential(db, p))
        .transpose()?
        .unwrap_or_default();
    ensure!(
        model.connection_id.is_some() || !changed || old_key.is_empty() || clear || key.is_some(),
        "更改 API 地址后，请重新填写密钥或移除旧密钥"
    );
    let count: i64 = db.query_row("SELECT count(*) FROM model_profiles", [], |r| r.get(0))?;
    ensure!(old.is_some() || count < 50, "最多保存 50 个模型连接");
    model.has_key = false;
    db.execute(
        "INSERT INTO model_profiles VALUES(?1,?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data",
        params![model.id, serde_json::to_string(&model)?],
    )?;
    if changed || clear || model.connection_id.is_some() {
        db.execute("DELETE FROM model_credentials WHERE id=?1", [&model.id])?;
    }
    if let Some(key) = key {
        db.execute("INSERT INTO model_credentials VALUES(?1,?2,?3) ON CONFLICT(id) DO UPDATE SET endpoint=excluded.endpoint,api_key=excluded.api_key", params![model.id, model.profile.endpoint, key])?;
    }
    if setting(db, "default-agent-model")?.is_none() && resolve(db, None, Some(&model.id)).is_ok() {
        set_default(db, &model.id)?;
    }
    Ok(())
}

pub fn set_default(db: &Connection, id: &str) -> Result<()> {
    resolve(db, None, Some(id))?;
    db.execute("INSERT INTO settings VALUES('default-agent-model',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [id])?;
    Ok(())
}

pub fn select(db: &Connection, project: &str, id: Option<&str>) -> Result<()> {
    if let Some(id) = id {
        resolve(db, None, Some(id))?;
    }
    db.execute("INSERT INTO agent_model_preferences VALUES(?1,?2) ON CONFLICT(project_id) DO UPDATE SET model_id=excluded.model_id", params![project, id])?;
    Ok(())
}

pub fn remove(db: &Connection, id: &str) -> Result<()> {
    get(db, id)?;
    db.execute("DELETE FROM model_profiles WHERE id=?1", [id])?;
    if setting(db, "default-agent-model")?.as_deref() == Some(id) {
        db.execute("DELETE FROM settings WHERE key='default-agent-model'", [])?;
    }
    Ok(())
}
