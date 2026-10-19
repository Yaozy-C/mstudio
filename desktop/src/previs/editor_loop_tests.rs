use super::{
    editor::{Editor, TOOL_NAME},
    editor_schema, files,
    tests::{cleanup, model, setup, shot},
};
use crate::{
    assistant::{
        config::Profile,
        harness::{self, Host, session::Session},
        model_io::{CompletionModel, Frame, ModelStream},
    },
    database::Store,
};
use rig_core::{
    completion::{CompletionRequest, CompletionResponse, ToolDefinition},
    message::{AssistantContent, Message, ToolCall, ToolFunction, ToolResultContent, UserContent},
};
use serde_json::{Value, json};
use std::{collections::VecDeque, sync::Mutex};
use tokio_util::sync::CancellationToken;
struct FileHost<'a> {
    store: &'a Store,
    editor: Mutex<Editor>,
    token: CancellationToken,
    cancel_after_write: bool,
}
impl Host for FileHost<'_> {
    fn token(&self) -> &CancellationToken {
        &self.token
    }
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![ToolDefinition::new(
            TOOL_NAME.try_into().unwrap(),
            editor_schema::DESCRIPTION,
            editor_schema::schema(),
        )]
    }
    fn parallel_safe(&self, _: &ToolCall) -> bool {
        false
    }
    fn record(&self, _: &str, _: Value) -> Result<(), String> {
        Ok(())
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        let result = self.editor.lock().unwrap().execute(
            self.store,
            "p",
            call.function.name.as_str(),
            Value::Object(call.function.arguments.clone()),
        );
        match result {
            Ok(text) => {
                if self.cancel_after_write {
                    self.token.cancel();
                }
                Value::String(text)
            }
            Err(error) => json!({"error":error.to_string()}),
        }
    }
}
struct Script<'a> {
    replies: Mutex<VecDeque<CompletionResponse>>,
    requests: Mutex<Vec<CompletionRequest>>,
    store: &'a Store,
}
impl CompletionModel for Script<'_> {
    async fn completion(
        &self,
        request: CompletionRequest,
    ) -> Result<CompletionResponse, rig_core::ProviderError> {
        let mut requests = self.requests.lock().unwrap();
        if requests.len() == 2 {
            let saved = files::read(&self.store.db.lock().unwrap(), "p", "s1")
                .unwrap()
                .unwrap();
            assert!(saved.code.starts_with("// persisted before final answer"));
        }
        requests.push(request);
        Ok(self
            .replies
            .lock()
            .unwrap()
            .pop_front()
            .expect("unexpected extra model request"))
    }
    async fn stream(
        &self,
        request: CompletionRequest,
    ) -> Result<ModelStream, rig_core::ProviderError> {
        let response = self.completion(request).await?;
        Ok(Box::pin(futures::stream::iter([Ok(Frame::Done(
            Box::new(response),
        ))])))
    }
}
fn reply(content: AssistantContent) -> CompletionResponse {
    CompletionResponse::new(
        vec![content],
        Default::default(),
        rig_core::message::Origin::new("test", "test", "test"),
        Value::Null,
    )
}
fn call(id: &str, args: Value) -> AssistantContent {
    AssistantContent::ToolCall(ToolCall::from_wire(
        id,
        ToolFunction::new(TOOL_NAME.try_into().unwrap(), args),
    ))
}
fn profile() -> Profile {
    Profile {
        endpoint: "http://localhost".into(),
        model: "test".into(),
        adapter: "openai-compatible".into(),
        context_window: None,
        limits: crate::models::context::Settings {
            compaction_headroom: Some(0),
            ..Default::default()
        },
        inputs: Default::default(),
    }
}
fn editor(store: &Store) -> Editor {
    let db = store.db.lock().unwrap();
    files::save(&db, store, "p", model("bag", "Bag"), 0).unwrap();
    let file = files::save(&db, store, "p", shot("s1", "bag"), 0).unwrap();
    Editor::new(&file, &files::list(&db, "p").unwrap())
}
#[tokio::test]
async fn native_error_result_allows_correction_and_success_writes_before_the_final_answer() {
    let store = setup();
    let host = FileHost {
        store: &store,
        editor: Mutex::new(editor(&store)),
        token: CancellationToken::new(),
        cancel_after_write: false,
    };
    let script = Script {
        store: &store,
        requests: Mutex::new(vec![]),
        replies: Mutex::new(VecDeque::from([
            reply(call(
                "bad",
                json!({"command":"create","path":"/scene.json","file_text":"no extra protocol"}),
            )),
            reply(call(
                "fix",
                json!({"command":"insert","path":"/scene.js","insert_line":0,"new_str":"// persisted before final answer"}),
            )),
            reply(AssistantContent::text("已修改")),
        ])),
    };
    let answer = harness::run(
        &script,
        &profile(),
        &host,
        Session::new(vec![Message::user("Change timing")]),
        true,
        "",
    )
    .await
    .unwrap();
    assert_eq!(answer, "已修改");
    let requests = script.requests.lock().unwrap();
    assert_eq!(requests.len(), 3);
    let result = requests[1]
        .chat_history
        .iter()
        .filter_map(|m| match m {
            Message::User { content } => content.iter().find_map(|v| match v {
                UserContent::ToolResult(r) => Some(r),
                _ => None,
            }),
            _ => None,
        })
        .next_back()
        .unwrap();
    assert!(result.is_error);
    assert!(
        matches!(&result.content[0],ToolResultContent::Text(text) if text.text.starts_with("Error: "))
    );
    let success = requests[2]
        .chat_history
        .iter()
        .filter_map(|m| match m {
            Message::User { content } => content.iter().find_map(|v| match v {
                UserContent::ToolResult(r) => Some(r),
                _ => None,
            }),
            _ => None,
        })
        .next_back()
        .unwrap();
    assert!(!success.is_error);
    assert!(
        matches!(&success.content[0],ToolResultContent::Text(text) if text.text.starts_with("The file /scene.js"))
    );
    assert!(
        !serde_json::to_string(&requests[2].chat_history)
            .unwrap()
            .contains("files are not ready to save")
    );
    drop(requests);
    drop(script);
    drop(host);
    cleanup(store);
}
#[tokio::test]
async fn cancellation_after_a_completed_write_retains_the_saved_file() {
    let store = setup();
    let host = FileHost {
        store: &store,
        editor: Mutex::new(editor(&store)),
        token: CancellationToken::new(),
        cancel_after_write: true,
    };
    let script = Script {
        store: &store,
        requests: Mutex::new(vec![]),
        replies: Mutex::new(VecDeque::from([reply(call(
            "save",
            json!({"command":"insert","path":"/scene.js","insert_line":0,"new_str":"// keep on cancellation"}),
        ))])),
    };
    assert!(
        harness::run(
            &script,
            &profile(),
            &host,
            Session::new(vec![Message::user("Edit")]),
            true,
            ""
        )
        .await
        .is_err()
    );
    assert_eq!(script.requests.lock().unwrap().len(), 1);
    assert!(
        files::read(&store.db.lock().unwrap(), "p", "s1")
            .unwrap()
            .unwrap()
            .code
            .starts_with("// keep on cancellation")
    );
    drop(script);
    drop(host);
    cleanup(store);
}
