use super::super::outcomes::*;
use super::super::session;
use super::support::call;
use serde_json::json;
#[test]
fn saved_but_incomplete_delegation_survives_replay_and_other_successes() {
    let result = json!({"ok":false,"applied":true,"childId":"child-1","stopReason":"step-limit"});
    let messages = vec![session::result_message(
        &call("c", "mstudio_delegate", json!({})),
        &result,
    )];
    let mut state = Outcomes::from_messages(&messages);
    assert!(state.notice().contains("已有修改已保存"));
    assert!(state.notice().contains("复查结果未确认"));
    state.observe(&json!({"ok":true,"childId":"other","stopReason":"completed"}));
    assert!(!state.notice().is_empty());
    state.observe(&json!({"ok":true,"childId":"child-1","stopReason":"completed"}));
    assert!(state.notice().is_empty());
}

#[tokio::test]
async fn final_answer_discloses_child_failure_even_when_model_claims_success() {
    use super::super::{Host, driver};
    use super::support::{TestHost, TestModel, profile, reply};
    use rig_core::{
        completion::ToolDefinition,
        message::{AssistantContent, Message, ToolCall},
    };
    use serde_json::Value;
    struct DelegateHost(TestHost);
    impl Host for DelegateHost {
        fn token(&self) -> &tokio_util::sync::CancellationToken {
            &self.0.token
        }
        fn record(&self, kind: &str, value: Value) -> Result<(), String> {
            self.0.record(kind, value)
        }
        fn definitions(&self) -> Vec<ToolDefinition> {
            vec![ToolDefinition {
                name: "mstudio_delegate".into(),
                description: "delegate".into(),
                parameters: json!({"type":"object"}),
            }]
        }
        fn parallel_safe(&self, _: &ToolCall) -> bool {
            false
        }
        async fn execute(&self, _: &ToolCall) -> Value {
            json!({"ok":false,"applied":true,"childId":"child-1","stopReason":"step-limit"})
        }
    }
    let host = DelegateHost(TestHost::default());
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Ok(reply(vec![AssistantContent::ToolCall(call(
            "d",
            "mstudio_delegate",
            json!({}),
        ))])),
        Ok(reply(vec![AssistantContent::text("所有转场完成")])),
    ]);
    let answer = driver::run(
        &model,
        &profile(),
        &host,
        session::Session::new(vec![Message::user("add transitions")]),
        false,
        "",
    )
    .await
    .unwrap();
    assert!(answer.contains("达到执行轮次上限"));
    assert!(answer.contains("已有修改已保存"));
    assert!(answer.contains("复查结果未确认"));
    let events = host.0.events.lock().unwrap();
    assert!(
        events
            .iter()
            .any(|(k, v)| k == "assistant/partial" && v["text"].as_str() == Some(answer.as_str()))
    );
}
