use super::{
    super::{run, session::Session},
    support::*,
};
use rig_core::message::{AssistantContent, Message};
use serde_json::json;
#[tokio::test]
async fn tool_work_can_cross_both_former_limits_and_finish_normally() {
    let host = TestHost::default();
    let model = TestModel::default();
    for index in 1..=65 {
        model
            .responses
            .lock()
            .unwrap()
            .push_back(Ok(reply(vec![AssistantContent::ToolCall(call(
                &format!("read-{index}"),
                "read",
                json!({}),
            ))])));
    }
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![AssistantContent::text("完成")])));
    let answer = run(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::user("do it")]),
        false,
        "",
    )
    .await
    .unwrap();
    assert_eq!(answer, "完成");
    assert_eq!(model.requests.lock().unwrap().len(), 66);
    let events = host.events.lock().unwrap();
    assert_eq!(
        events
            .iter()
            .filter(|(kind, _)| kind == "tool/result")
            .count(),
        65
    );
    assert!(
        events
            .iter()
            .any(|(kind, value)| kind == "step/end" && value["step"] == 66)
    );
    assert!(
        !events
            .iter()
            .any(|(kind, value)| kind == "model/stop" && value["stopReason"] == "step-limit")
    );
}

#[tokio::test]
async fn tool_errors_return_to_model_for_correction_without_edit_specific_stop() {
    use super::super::Host;
    use rig_core::{completion::ToolDefinition, message::ToolCall};
    use serde_json::Value;
    struct Edits(TestHost);
    impl Host for Edits {
        fn token(&self) -> &tokio_util::sync::CancellationToken {
            &self.0.token
        }
        fn record(&self, kind: &str, value: Value) -> Result<(), String> {
            self.0.record(kind, value)
        }
        fn definitions(&self) -> Vec<ToolDefinition> {
            vec![ToolDefinition {
                name: "mstudio_edit".into(),
                description: "Edit".into(),
                parameters: json!({"type":"object","properties":{"id":{"type":"string"}},"required":["id"]}),
            }]
        }
        fn parallel_safe(&self, _: &ToolCall) -> bool {
            false
        }
        async fn execute(&self, call: &ToolCall) -> Value {
            if call.function.arguments["id"] == "correct" {
                json!({"applied":true})
            } else {
                json!({"error":"Unknown target", "outcome":"not_executed"})
            }
        }
    }
    let host = Edits(TestHost::default());
    let model = TestModel::default();
    for (index, id) in ["wrong-a", "wrong-b", "wrong-c", "correct"]
        .iter()
        .enumerate()
    {
        model
            .responses
            .lock()
            .unwrap()
            .push_back(Ok(reply(vec![AssistantContent::ToolCall(call(
                &format!("edit-{index}"),
                "mstudio_edit",
                json!({"id":id}),
            ))])));
    }
    model
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![AssistantContent::text("Saved")])));
    assert_eq!(
        run(
            &model,
            &profile(),
            &host,
            Session::new(vec![Message::user("edit")]),
            false,
            ""
        )
        .await
        .unwrap(),
        "Saved"
    );
    let results: Vec<_> = host
        .0
        .events
        .lock()
        .unwrap()
        .iter()
        .filter(|(kind, _)| kind == "tool/result")
        .map(|(_, value)| value.clone())
        .collect();
    assert_eq!(results.len(), 4);
    assert_eq!(results[3]["value"]["applied"], true);
}
