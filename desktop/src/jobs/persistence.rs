use super::*;
// Reserve before network I/O. Repeating one submission never submits another paid request.
pub fn reserve(store: &Store, job: &Value) -> Result<bool> {
    let inserted = write(store, job, true)?;
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

pub fn save(store: &Store, job: &Value) -> Result<()> {
    ensure!(write(store, job, false)? == 1, "项目已删除");
    Ok(())
}
fn write(store: &Store, job: &Value, reserve: bool) -> Result<usize> {
    let mut db = store.db.lock().unwrap();
    let tx = db.transaction()?;
    let mut value = job.clone();
    store.normalize_paths(&mut value);
    let id = job["id"].as_str().context("缺少任务 ID")?;
    let sql = if reserve {
        "INSERT OR IGNORE INTO jobs SELECT ?1,?2,'{}' WHERE EXISTS(SELECT 1 FROM projects WHERE id=?2)"
    } else {
        "INSERT INTO jobs SELECT ?1,?2,'{}' WHERE EXISTS(SELECT 1 FROM projects WHERE id=?2) ON CONFLICT(id) DO UPDATE SET data=excluded.data"
    };
    let inserted = tx.execute(sql, rusqlite::params![id, job["projectId"].as_str()])?;
    if inserted == 1 {
        crate::database::blobs::pack(&tx, crate::database::blobs::Owner::Job(id), &mut value)?;
        tx.execute(
            "UPDATE jobs SET data=?1 WHERE id=?2",
            rusqlite::params![value.to_string(), id],
        )?;
        crate::database::blobs::collect(&tx)?;
    }
    tx.commit()?;
    if let Err(error) = crate::project_storage::resume_cleanup(&db, &store.media_root()) {
        eprintln!("{error}");
    }
    Ok(inserted)
}
pub fn get(store: &Store, id: &str) -> Result<Value> {
    let db = store.db.lock().unwrap();
    let raw: String = db.query_row("SELECT data FROM jobs WHERE id=?1", [id], |r| r.get(0))?;
    crate::database::blobs::hydrate(
        &db,
        crate::database::blobs::Owner::Job(id),
        serde_json::from_str(&raw)?,
    )
}
