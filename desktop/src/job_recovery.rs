use crate::{app_error, database::Store, jobs};
use anyhow::{Result, ensure};
use serde_json::{Value, json};
use tauri::Manager;

pub fn recover_interrupted_submissions(db: &rusqlite::Connection) -> Result<()> {
    let mut error = app_error::AppError::new(
        "SUBMISSION_UNKNOWN",
        "submit",
        "应用退出前未收到生成服务的提交确认",
    );
    error.outcome = Some("unknown".into());
    db.execute("UPDATE jobs SET data=json_set(data,'$.status','UNKNOWN','$.error',?1) WHERE json_extract(data,'$.status')='SUBMITTING'", [error.to_string()])?;
    Ok(())
}

pub fn resolve_unknown(job: &mut Value, confirmed: bool) -> Result<()> {
    ensure!(confirmed, "请先核实原任务没有执行或已经终止");
    ensure!(
        matches!(job["status"].as_str(), Some("UNKNOWN" | "SUBMITTING")),
        "任务状态已更新，请重新查看"
    );
    job["status"] = json!("FAILED");
    job["error"] = json!(
        app_error::AppError::new(
            "GENERATION_FAILED",
            "manual-resolution",
            "用户已在服务商核实原任务未执行或已终止"
        )
        .to_string()
    );
    Ok(())
}
#[tauri::command]
pub async fn resolve_unknown_job(
    app: tauri::AppHandle,
    id: String,
    confirmed: bool,
) -> Result<Value, String> {
    async {
        let _job_guard = crate::job_locks::acquire(&id).await;
        let store = app.state::<Store>();
        ensure!(confirmed, "请先核实原任务没有执行或已经终止");
        let mut job = match jobs::get(&store, &id) {
            Ok(job) => job,
            Err(error)
                if matches!(
                    error.downcast_ref::<rusqlite::Error>(),
                    Some(rusqlite::Error::QueryReturnedNoRows)
                ) =>
            {
                return Ok(json!({"id":id,"status":"FAILED"}));
            }
            Err(error) => return Err(error),
        };
        resolve_unknown(&mut job, confirmed)?;
        jobs::save(&store, &job)?;
        Ok::<_, anyhow::Error>(job)
    }
    .await
    .map_err(|e| app_error::wire(e, "JOB_SYNC_FAILED", "manual-resolution"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn unresolved_and_running_jobs_cannot_be_reset() {
        let mut job = json!({"id":"original","status":"UNKNOWN"});
        assert!(resolve_unknown(&mut job, false).is_err());
        assert_eq!(job["status"], "UNKNOWN");
        resolve_unknown(&mut job, true).unwrap();
        assert_eq!(job["status"], "FAILED");
        assert_eq!(job["id"], "original");
        for status in ["IN_PROGRESS", "IN_QUEUE", "CANCEL_REQUESTED", "COMPLETED"] {
            assert!(resolve_unknown(&mut json!({"status":status}), true).is_err());
        }
    }
}
