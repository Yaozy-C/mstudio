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
        let db = store.conn()?;
        let connection = crate::models::connections::get(&db, id)?;
        ensure!(
            connection.endpoint == job["connectionEndpoint"]
                && (connection.kind == provider.id()
                    || (connection.kind == "codex" && provider.id() == "codex-image")),
            "服务连接已变化，不向原任务地址发送新凭据"
        );
        return crate::models::connections::key(&db, id);
    }
    Err(
        crate::app_error::AppError::new("VALIDATION_FAILED", "connection", "生成任务缺少服务连接")
            .into(),
    )
}

pub fn save(store: &Store, job: &Value) -> Result<()> {
    ensure!(write(store, job, false)? == 1, "项目已删除");
    Ok(())
}
fn write(store: &Store, job: &Value, reserve: bool) -> Result<usize> {
    let mut db = store.conn()?;
    let tx = db.transaction()?;
    let mut value = job.clone();
    store.normalize_paths(&mut value);
    crate::project_storage::delete_media::sanitize_job(&tx, &mut value)?;
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
    // The outbox already owns SUBMITTING before reserve. A reservation must not
    // depend on rendering a project; only later provider updates reconcile it.
    let reconciliation = if inserted == 1 && !reserve {
        let error = super::reconciliation::apply(&tx, &mut value)?;
        super::reconciliation::record(&tx, id, error.as_ref().map(|e| e.to_string()).as_deref())?;
        error
    } else {
        None
    };
    tx.commit()?;
    if let Some(project) = job["projectId"].as_str() {
        store.project_changed(project);
    }
    if let Err(error) = crate::project_storage::resume_cleanup(&db, &store.media_root()) {
        eprintln!("{error}");
    }
    if let Some(error) = reconciliation {
        return Err(error.into());
    }
    Ok(inserted)
}
pub fn get(store: &Store, id: &str) -> Result<Value> {
    let db = store.conn()?;
    let raw: String = db.query_row("SELECT data FROM jobs WHERE id=?1", [id], |r| r.get(0))?;
    crate::database::blobs::hydrate(
        &db,
        crate::database::blobs::Owner::Job(id),
        serde_json::from_str(&raw)?,
    )
}
