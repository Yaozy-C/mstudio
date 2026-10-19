//! A dedicated previs Agent: fresh session, one file editor, no project Agent context.
use super::{chat::Request, editor::Editor, editor_schema};
use crate::{
    assistant::harness::{self, Host, session::Session},
    models::Model,
};
use anyhow::{Result, anyhow};
use rig_core::{
    completion::ToolDefinition,
    message::{Message, ToolCall},
};
use serde_json::{Value, json};
use std::sync::Mutex;
use tauri::Emitter;
use tokio_util::sync::CancellationToken;

pub struct PrevisAgent<'a> {
    pub editor: Mutex<Editor>,
    pub app: &'a tauri::AppHandle,
    pub request: &'a Request,
    pub token: &'a CancellationToken,
}
impl PrevisAgent<'_> {
    pub async fn run(&self, model: &Model, key: &str, task: Message) -> Result<String> {
        let transport =
            crate::assistant::provider::text_model(&model.profile, key).map_err(|e| anyhow!(e))?;
        let session = Session::new(vec![Message::system(editor_schema::CONTRACT), task]);
        harness::run(&transport, &model.profile, self, session, true, key)
            .await
            .map_err(|e| anyhow!(e))
    }
}
impl Host for PrevisAgent<'_> {
    fn token(&self) -> &CancellationToken {
        self.token
    }
    fn tool_argument_limit(&self) -> usize {
        4 * super::editor::MAX_BYTES
    }
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition::new(
            super::editor::TOOL_NAME.try_into().unwrap(),
            editor_schema::DESCRIPTION,
            editor_schema::schema(),
        )]
    }
    fn parallel_safe(&self, _: &ToolCall) -> bool {
        false
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        // Session messages, provider payloads and tool source stay private to this run.
        let visible = match kind {
            "assistant/partial"
            | "assistant/progress"
            | "assistant/thinking"
            | "assistant/reasoning" => value,
            "tool/call" => json!({"callId":value["callId"],"name":value["name"],
                "arguments":visible_arguments(&value["arguments"])}),
            "tool/result" => json!({"callId":value["callId"],"name":value["name"],
                "isError":value["isError"],"result":{"error":value["result"]["error"]}}),
            _ => return Ok(()),
        };
        // Closing a view must not stop the Agent or prevent its file save.
        let _ = self.app.emit(
            "previs-agent-progress",
            json!({
                "projectId":self.request.project_id,"fileId":self.request.file_id,
                "requestId":self.request.request_id,"kind":kind,"value":visible,
            }),
        );
        Ok(())
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        use tauri::Manager;
        let store = self.app.state::<crate::database::Store>();
        let result = async {
            let _guard = crate::project_storage::working(&store, &self.request.project_id).await?;
            anyhow::ensure!(
                !self.token.is_cancelled(),
                "Task stopped; completed file edits have been saved"
            );
            self.editor.lock().unwrap().execute(
                &store,
                &self.request.project_id,
                call.function.name.as_str(),
                Value::Object(call.function.arguments.clone()),
            )
        }
        .await;
        match result {
            Ok(text) => Value::String(text),
            Err(error) => json!({"error":error.to_string()}),
        }
    }
}

fn visible_arguments(input: &Value) -> Value {
    Value::Object(
        ["command", "path", "view_range", "insert_line"]
            .into_iter()
            .filter_map(|key| input.get(key).map(|value| (key.into(), value.clone())))
            .collect(),
    )
}

#[cfg(test)]
#[test]
fn activity_keeps_actual_read_ranges_without_publishing_source_code() {
    let input = json!({"command":"view","path":"/scene.js","view_range":[200,500],
        "file_text":"private source","old_str":"private source","new_str":"private source"});
    assert_eq!(
        visible_arguments(&input),
        json!({"command":"view","path":"/scene.js","view_range":[200,500]})
    );
    assert_eq!(
        visible_arguments(&json!({"command":"view","path":"/scene.js"})),
        json!({"command":"view","path":"/scene.js"})
    );
}
