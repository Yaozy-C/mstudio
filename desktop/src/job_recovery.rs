use crate::app_error;
use anyhow::Result;
use serde_json::{Value, json};

pub fn recover_interrupted_submissions(db: &rusqlite::Connection) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    let interrupted = {
        let mut statement = tx.prepare("SELECT id,json_extract(data,'$.status'),json_extract(data,'$.requestId'),json_extract(data,'$.error') FROM jobs WHERE json_extract(data,'$.status') IN ('SUBMITTING','UNKNOWN')")?;
        statement
            .query_map([], |r| {
                Ok((
                    r.get::<_, String>(0)?,
                    r.get::<_, String>(1)?,
                    r.get::<_, Option<String>>(2)?,
                    r.get::<_, Option<String>>(3)?,
                ))
            })?
            .collect::<rusqlite::Result<Vec<_>>>()?
    };
    for (id, status, request_id, error) in interrupted {
        // A received provider handle can still be polled after a restart.
        if status == "SUBMITTING" && request_id.is_some_and(|id| !id.is_empty()) {
            tx.execute(
                "UPDATE jobs SET data=json_set(data,'$.status','IN_QUEUE') WHERE id=?1",
                [&id],
            )?;
            continue;
        }
        let details = error
            .as_deref()
            .unwrap_or("应用退出前未收到生成服务的提交确认");
        let error = serde_json::from_str::<app_error::AppError>(details)
            .map(anyhow::Error::new)
            .unwrap_or_else(|_| anyhow::anyhow!("{details}"));
        let mut update = json!({});
        submission_failed(&mut update, &error);
        tx.execute("UPDATE jobs SET data=json_set(json_remove(data,'$.sync'),'$.status','FAILED','$.error',?2) WHERE id=?1", rusqlite::params![id, update["error"].as_str()])?;
    }
    tx.commit()?;
    Ok(())
}

pub(crate) fn submission_failed(job: &mut Value, error: &anyhow::Error) {
    let mut issue = app_error::AppError::from_error(error, "GENERATION_FAILED", "submit");
    if issue.code == "SUBMISSION_UNKNOWN" {
        issue.code = "GENERATION_FAILED".into();
    }
    if issue.outcome.as_deref() != Some("rejected") {
        issue.outcome = Some("failed".into());
    }
    job["status"] = json!("FAILED");
    job["error"] = json!(issue.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn submission_errors_fail_without_overwriting_the_provider_reason() {
        for error in [
            anyhow::anyhow!("size 须为 WIDTHxHEIGHT：宽高为 16 的倍数"),
            app_error::AppError::http(422, "fal", "invalid size").into(),
            app_error::AppError::http(503, "submit", "service unavailable").into(),
            app_error::AppError::http(408, "submit", "request timeout").into(),
        ] {
            let original = app_error::AppError::from_error(&error, "GENERATION_FAILED", "submit");
            let mut job = json!({"id":"job","status":"SUBMITTING","input":{"size":"123x456"}});
            submission_failed(&mut job, &error);
            let saved: app_error::AppError =
                serde_json::from_str(job["error"].as_str().unwrap()).unwrap();
            assert_eq!(job["status"], "FAILED");
            assert_eq!(job["input"]["size"], "123x456");
            assert_eq!(saved.code, original.code);
            assert_eq!(saved.details, original.details);
            assert_eq!(saved.http_status, original.http_status);
            assert_ne!(saved.outcome.as_deref(), Some("unknown"));
        }
    }
    #[test]
    fn restart_converts_stuck_submissions_and_preserves_running_jobs() {
        let db = rusqlite::Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE jobs(id TEXT PRIMARY KEY, data TEXT NOT NULL)")
            .unwrap();
        let mut legacy =
            app_error::AppError::new("SUBMISSION_UNKNOWN", "submit", "size 须为 WIDTHxHEIGHT");
        legacy.outcome = Some("unknown".into());
        for (id, job) in [
            (
                "legacy",
                json!({"status":"UNKNOWN","error":legacy.to_string(),"sync":{"paused":true},"input":{"size":"123x456"}}),
            ),
            ("interrupted", json!({"status":"SUBMITTING"})),
            (
                "accepted",
                json!({"status":"SUBMITTING","requestId":"remote"}),
            ),
            (
                "running",
                json!({"status":"IN_PROGRESS","requestId":"remote"}),
            ),
            (
                "done",
                json!({"status":"COMPLETED","outputs":[{"kind":"image","url":"synthetic.png"}]}),
            ),
        ] {
            db.execute(
                "INSERT INTO jobs VALUES (?1,?2)",
                rusqlite::params![id, job.to_string()],
            )
            .unwrap();
        }
        recover_interrupted_submissions(&db).unwrap();
        recover_interrupted_submissions(&db).unwrap();
        let read = |id: &str| -> Value {
            let raw: String = db
                .query_row("SELECT data FROM jobs WHERE id=?1", [id], |r| r.get(0))
                .unwrap();
            serde_json::from_str(&raw).unwrap()
        };
        let job = read("legacy");
        let issue: app_error::AppError =
            serde_json::from_str(job["error"].as_str().unwrap()).unwrap();
        assert_eq!(job["status"], "FAILED");
        assert_eq!(job["input"]["size"], "123x456");
        assert!(job.get("sync").is_none());
        assert_eq!(issue.code, "GENERATION_FAILED");
        assert_eq!(issue.details, "size 须为 WIDTHxHEIGHT");
        assert_eq!(issue.outcome.as_deref(), Some("failed"));
        assert_eq!(read("interrupted")["status"], "FAILED");
        assert_eq!(read("accepted")["status"], "IN_QUEUE");
        assert_eq!(read("accepted")["requestId"], "remote");
        assert_eq!(read("running")["status"], "IN_PROGRESS");
        assert_eq!(read("done")["outputs"][0]["url"], "synthetic.png");
    }
}
