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
