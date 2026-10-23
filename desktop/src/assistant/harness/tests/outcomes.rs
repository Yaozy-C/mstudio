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
        fn definitions(&self) -> Result<Vec<ToolDefinition>, String> {
            Ok({
                vec![ToolDefinition {
                    name: "mstudio_delegate".try_into().expect("nonempty tool name"),
                    description: "delegate".into(),
                    parameters: json!({"type":"object"}),
                }]
            })
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

#[test]
fn normal_child_exit_does_not_hide_missing_generation_and_recovery_clears_it() {
    let partial = json!({"ok":true,"childId":"first","stopReason":"completed","delivery":{
        "complete":false,"items":[{"targetId":"s3","kind":"video","created":false}]},
        "generationTasks":[{"id":"five","targetId":"s5","kind":"video","status":"AWAITING_CONFIRMATION"}]});
    let mut outcomes = Outcomes::default();
    outcomes.observe(&partial);
    assert!(outcomes.notice().contains("s3"));
    assert!(outcomes.recovery().is_some());
    assert!(outcomes.recovery().is_none()); // Do not loop when a real blocker remains.
    outcomes.observe(&json!({"childId":"second","ok":true,"stopReason":"completed",
        "generationTasks":[{"id":"three-four","targetId":"s3","kind":"video","status":"AWAITING_CONFIRMATION"}]}));
    assert!(outcomes.notice().is_empty());
}

#[tokio::test]
async fn driver_reconciles_missing_receipts_before_accepting_final_answer() {
    use super::super::driver;
    use super::support::{TestHost, TestModel, profile, reply};
    use rig_core::message::{AssistantContent, Message};
    let host = TestHost::default();
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Ok(reply(vec![AssistantContent::text("全部完成")])),
        Ok(reply(vec![AssistantContent::text(
            "镜头3缺少必需的参考素材，任务尚未创建。",
        )])),
    ]);
    let messages = vec![
        Message::user("创建视频任务"),
        session::result_message(
            &call("d", "mstudio_delegate", json!({})),
            &json!({"ok":true,"childId":"c","stopReason":"completed","delivery":{"complete":false,
            "items":[{"targetId":"s3","kind":"video","created":false}]}}),
        ),
    ];
    let answer = driver::run(
        &model,
        &profile(),
        &host,
        session::Session::new(messages),
        false,
        "",
    )
    .await
    .unwrap();
    assert_eq!(model.requests.lock().unwrap().len(), 2);
    assert!(answer.contains("任务尚未创建"));
    assert!(answer.contains("交付未齐"));
}

#[test]
fn asynchronous_notice_tracks_missing_work_without_repeated_recovery_loops() {
    use rig_core::message::Message;
    let value = json!({"outputRef":{"id":"child"},"delivery":{"complete":false,
        "items":[{"targetId":"s3","kind":"video","created":false}]}});
    let message = Message::user(format!(
        "Subagent child settled this turn with status completed. Final message: {value}"
    ));
    let mut outcomes = Outcomes::from_messages(&[message]);
    assert!(outcomes.recovery().is_some());
    outcomes.observe(&json!({"childId":"retry","delivery":value["delivery"]}));
    assert!(outcomes.recovery().is_none());
    outcomes.observe(&json!({"generationTasks":[{"id":"t","targetId":"s3","kind":"video"}]}));
    assert!(outcomes.notice().is_empty());
    outcomes.observe_messages(&[Message::user(
        "Subagent old settled this turn with status completed. Final message: []",
    )]);
}
