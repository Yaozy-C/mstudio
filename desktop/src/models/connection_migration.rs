use super::*;
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
    let fal_key = super::super::setting(&tx, "fal-key")?.unwrap_or_default();
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
        if model.connection_id.is_some() {
            continue;
        }
        let id = if model.plugin == "codex-image" {
            migrate_one(&tx, "Codex", "codex", "codex://local", "")?
        } else if model.plugin == "fal" {
            fal_id.clone().unwrap()
        } else {
            let secret =
                super::super::setting(&tx, &format!("media-key:{}", model.id))?.unwrap_or_default();
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
