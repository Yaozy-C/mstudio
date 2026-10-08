use super::*;
use crate::assistant::harness::{
    self, Host,
    session::Session,
    tool_loading::{LOAD, LoadedTools},
};
use rig_core::{
    completion::ToolDefinition,
    message::{Message, ToolCall},
};
use tokio_util::sync::CancellationToken;

#[derive(Default)]
struct LoadingHost {
    scope: LoadedTools,
    token: CancellationToken,
    events: Mutex<Vec<(String, Value)>>,
    saved: Mutex<Option<String>>,
}
impl LoadingHost {
    fn available(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition {
            name: "mstudio_test_save_prompt".into(),
            description: "Save a synthetic test prompt; no real project is changed.".into(),
            parameters: json!({"type":"object","properties":{"prompt":{"type":"string"}},"required":["prompt"],"additionalProperties":false}),
        }]
    }
}
impl Host for LoadingHost {
    fn token(&self) -> &CancellationToken {
        &self.token
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        self.events.lock().unwrap().push((kind.into(), value));
        Ok(())
    }
    fn definitions(&self) -> Vec<ToolDefinition> {
        self.scope.definitions(self.available())
    }
    fn parallel_safe(&self, _: &ToolCall) -> bool {
        false
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        match call.function.name.as_str() {
            LOAD => self.scope.load(&self.available(), &call.function.arguments),
            "mstudio_test_save_prompt" => {
                *self.saved.lock().unwrap() = call.function.arguments["prompt"]
                    .as_str()
                    .map(str::to_owned);
                json!({"ok":true,"saved":call.function.arguments["prompt"]})
            }
            _ => panic!("unexpected tool execution"),
        }
    }
}
#[tokio::test]
#[ignore = "uses local ChatGPT account through the real harness driver and scheduler; synthetic save only"]
async fn live_codex_loads_tools_then_continues_with_the_new_schema() {
    let rows = crate::models::codex_connection::model_list().await.unwrap();
    let row = rows
        .iter()
        .find(|r| r["isDefault"] == true)
        .unwrap_or(&rows[0]);
    let model_id = row["id"].as_str().unwrap().to_owned();
    let model = CodexModel::new(model_id.clone());
    let host = LoadingHost::default();
    let profile = crate::assistant::config::Profile {
        endpoint: "codex://local".into(),
        model: model_id,
        adapter: "codex".into(),
        context_window: None,
        inputs: Default::default(),
    };
    let session = Session::new(vec![Message::user(
        "Save the exact synthetic prompt 'Slow push'. Discover and load the needed tool if necessary. After the save succeeds, reply exactly MSTUDIO_LOADING_OK. This is an integration test.",
    )]);
    let answer = harness::run(&model, &profile, &host, session, true, "")
        .await
        .unwrap();
    assert!(answer.contains("MSTUDIO_LOADING_OK"));
    assert_eq!(host.saved.lock().unwrap().as_deref(), Some("Slow push"));
    let events = host.events.lock().unwrap();
    let usage = events
        .iter()
        .rev()
        .find(|(kind, _)| kind == "request/usage")
        .unwrap();
    assert!(usage.1["inputTokens"].as_u64().unwrap() > 0);
    assert!(usage.1["contextWindow"].as_u64().unwrap() > 0);
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "compaction/start" || kind == "image/offload")
    );
    println!("Harness Codex usage: {}", usage.1);
    let results: Vec<_> = events
        .iter()
        .filter(|(k, _)| k == "tool/result")
        .map(|(_, v)| v)
        .collect();
    assert!(results.len() >= 2, "must load then save: {results:?}");
    assert_eq!(results[0]["name"], LOAD);
    assert!(results[0]["result"]["loaded"].is_array());
    assert_eq!(results.last().unwrap()["name"], "mstudio_test_save_prompt");
    assert!(
        results[..results.len() - 1]
            .iter()
            .all(|v| v["name"] == LOAD)
    );
    assert!(results.iter().all(|v| v["isError"] == false));
}
