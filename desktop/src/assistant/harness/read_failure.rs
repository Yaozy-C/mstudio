use serde_json::{Value, json};

pub(super) fn missing(error: &anyhow::Error) -> bool {
    matches!(
        error.downcast_ref::<rusqlite::Error>(),
        Some(rusqlite::Error::QueryReturnedNoRows)
    )
}

pub(super) fn result(error: anyhow::Error, absent: &str, failed: &str) -> Value {
    let code = if missing(&error) { absent } else { failed };
    json!({"error":error.to_string(),"code":code})
}
