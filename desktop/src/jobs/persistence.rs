use super::*;
// Reserve before network I/O. Repeating one submission never submits another paid request.
pub fn reserve(store: &Store, job: &Value) -> Result<bool> {
    let inserted = store.db.lock().unwrap().execute(
        "INSERT OR IGNORE INTO jobs SELECT ?1,?2,?3 WHERE EXISTS(SELECT 1 FROM projects WHERE id=?2)",
        rusqlite::params![
            job["id"].as_str(),
            job["projectId"].as_str(),
            job.to_string()
        ],
    )?;
    if inserted == 0 {
        let previous = get(store, job["id"].as_str().context("缺少任务 ID")?)?;
        ensure!(
            previous["projectId"] == job["projectId"]
                && previous["input"] == job["input"]
                && previous["endpoint"] == job["endpoint"]
                && previous["providerConfig"] == job["providerConfig"]
                && previous["connectionId"] == job["connectionId"]
                && previous["connectionEndpoint"] == job["connectionEndpoint"]
                && previous["providerId"]
                    .as_str()
                    .ok_or_else(|| anyhow::anyhow!("任务缺少服务供应商"))?
                    == job["providerId"]
                        .as_str()
                        .ok_or_else(|| anyhow::anyhow!("任务缺少服务供应商"))?,
            "运行编号已被其他参数占用"
        );
    }
    Ok(inserted == 1)
}

pub(super) fn credential(store: &Store, job: &Value) -> Result<String> {
    let provider = crate::model_adapters::for_job(job)?;
    if let Some(id) = job["connectionId"].as_str() {
        let db = store.db.lock().unwrap();
        let connection = crate::models::connections::get(&db, id)?;
        ensure!(
            connection.endpoint == job["connectionEndpoint"] && connection.kind == provider.id(),
            "服务连接已变化，不向原任务地址发送新凭据"
        );
        return crate::models::connections::key(&db, id);
    }
    if provider.id() == "codex-image" {
        return Ok(String::new());
    }
    if provider.id() == "fal" {
        return crate::models::connections::legacy_fal_key(&store.db.lock().unwrap());
    }
    let id = job["mediaModelId"].as_str().context("任务缺少模型 ID")?;
    let models = crate::models::media::read(&store.db.lock().unwrap())?;
    let model = models
        .iter()
        .find(|m| m.id == id)
        .context("模型已移除，无法继续查询该任务")?;
    ensure!(
        model.endpoint == job["endpoint"] && model.plugin == job["providerId"],
        "服务地址已变更，不向旧地址发送新凭据"
    );
    crate::models::connections::media_key(&store.db.lock().unwrap(), model)
}
