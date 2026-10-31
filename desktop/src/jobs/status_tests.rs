use super::*;
use crate::{app_error::AppError, model_adapters::ProviderUpdate};

fn job() -> Value {
    json!({"id":"job","status":"IN_PROGRESS","requestId":"remote","input":{"size":"1080x1920"}})
}

#[test]
fn invalid_provider_states_and_missing_results_finish_with_an_error() {
    for value in [
        json!({"status":"UNKNOWN"}),
        json!({"status":"ERROR"}),
        json!({"status":"unexpected"}),
        json!({"status":"COMPLETED"}),
    ] {
        let mut saved = job();
        apply_refresh(
            &mut saved,
            Ok(serde_json::from_value::<ProviderUpdate>(value).unwrap()),
        )
        .unwrap();
        assert_eq!(saved["status"], "FAILED");
        assert_eq!(saved["requestId"], "remote");
        assert_eq!(saved["input"]["size"], "1080x1920");
        let issue: AppError = serde_json::from_str(saved["error"].as_str().unwrap()).unwrap();
        assert_eq!(issue.code, "GENERATION_FAILED");
        assert_eq!(issue.outcome.as_deref(), Some("failed"));
        assert!(!issue.details.is_empty());
    }
}

#[test]
fn permanent_provider_errors_finish_instead_of_pausing_tracking() {
    for status in [400, 404, 410, 422] {
        let mut saved = job();
        apply_refresh(
            &mut saved,
            Err(AppError::http(status, "status", "provider reason").into()),
        )
        .unwrap();
        assert_eq!(saved["status"], "FAILED");
        let issue: AppError = serde_json::from_str(saved["error"].as_str().unwrap()).unwrap();
        assert_eq!(issue.http_status, Some(status));
        assert_eq!(issue.details, "provider reason");
        assert_eq!(issue.outcome.as_deref(), Some("failed"));
    }
}

#[test]
fn temporary_polling_errors_preserve_the_running_job_for_status_retry() {
    for status in [401, 403, 429, 500, 503] {
        let mut saved = job();
        let before = saved.clone();
        assert!(
            apply_refresh(
                &mut saved,
                Err(AppError::http(status, "status", "retry status query").into())
            )
            .is_err()
        );
        assert_eq!(saved, before);
    }
}

#[test]
fn successful_status_updates_clear_old_errors_and_cancellation_waits_for_the_result() {
    let mut saved = job();
    saved["status"] = json!("CANCEL_REQUESTED");
    saved["error"] = json!("previous polling failure");
    apply_refresh(
        &mut saved,
        Ok(serde_json::from_value(json!({"status":"IN_PROGRESS"})).unwrap()),
    )
    .unwrap();
    assert_eq!(saved["status"], "CANCEL_REQUESTED");
    assert!(saved.get("error").is_none());
    apply_refresh(
        &mut saved,
        Ok(serde_json::from_value(json!({"status":"FAILED","error":"provider failed"})).unwrap()),
    )
    .unwrap();
    assert_eq!(saved["status"], "FAILED");
    assert_eq!(saved["error"], "provider failed");
}
