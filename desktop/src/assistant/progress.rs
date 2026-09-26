//! Persist authoritative Agent events and project their UI progress.
use super::{journal, tools::ProjectTool};
use crate::database::Store;
use serde_json::{Value, json};
use tauri::{Emitter, Manager};

impl ProjectTool {
    pub fn record(&self, kind: &str, payload: Value) -> Result<(), String> {
        journal::append(
            &self.app.state::<Store>(),
            &self.project,
            &self.turn,
            kind,
            payload.clone(),
        )
        .map_err(|e| e.to_string())?;
        if kind.starts_with("session/") {
            return Ok(());
        }
        let _ = self.app.emit(
            "agent-progress",
            json!({"projectId":self.project,"turnId":self.turn,"kind":kind,"payload":if kind == "tool/result" { json!({"callId":payload["callId"],"name":payload["name"],"result":payload["result"],"isError":payload["isError"]}) } else { payload }}),
        );
        Ok(())
    }
}
