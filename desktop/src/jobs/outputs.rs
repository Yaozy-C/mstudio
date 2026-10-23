use anyhow::Result;
use serde_json::Value;

pub(crate) fn count(job: &Value) -> Result<usize> {
    let completed = job["status"] == "COMPLETED";
    match job.get("outputCount") {
        None if !completed => Ok(0),
        Some(value) if value.as_u64().is_some_and(|n| !completed || n > 0) => {
            Ok(usize::try_from(value.as_u64().unwrap())?)
        }
        _ => Err(crate::app_error::AppError::new(
            "VALIDATION_FAILED",
            "outputs",
            "生成任务缺少有效的实际输出数量",
        )
        .into()),
    }
}
