use super::{
    super::{Host, budget, run, session::Session},
    support::*,
};
use rig_core::message::{AssistantContent, Message};
use serde_json::json;

#[test]
fn codex_unknown_window_is_not_32k_and_delayed_usage_prices_only_its_prefix() {
    let host = TestHost::default();
    let mut route = profile();
    route.adapter = "codex".into();
    let mut session = Session::new(vec![Message::user("request"), Message::assistant("call")]);
    assert_eq!(route.context_window(), None);
    assert_eq!(budget::input_budget(&session, &route), None);
    let measured = budget::raw_pressure(&session, &host);
    session.messages.extend([
        Message::user("tool result"),
        Message::assistant("next call"),
    ]);
    budget::anchor_response(
        &mut session,
        &route,
        budget::header(&route, &host.definitions()),
        &json!({"tokenUsage":{"modelContextWindow":258400,"last":{"totalTokens":40000},"total":{"totalTokens":900000}},"contextMessageCount":2}),
        900000,
    );
    assert_eq!(budget::input_budget(&session, &route), Some(206720));
    assert_eq!(
        budget::pressure(&session, &route, &host),
        40000 + budget::raw_pressure(&session, &host) - measured
    );
    assert!(!budget::over_budget(&session, &route, &host));
    route.context_window = Some(8192);
    assert_eq!(budget::input_budget(&session, &route), Some(6553));
    route.context_window = None;
    session.provider_context_window = Some(8192);
    assert!(budget::over_budget(&session, &route, &host));
}

#[tokio::test]
async fn native_capacity_prevents_premature_pruning_across_real_driver_steps() {
    let host = TestHost::default();
    let model = TestModel::default();
    let mut route = profile();
    route.adapter = "codex".into();
    let messages = vec![
        Message::System {
            content: "rules".into(),
        },
        Message::Assistant {
            id: None,
            content: vec![AssistantContent::ToolCall(call("old", "read", json!({})))],
        },
        Message::tool_result("old", "read", "x".repeat(120000)),
        Message::assistant("read"),
        Message::user("continue"),
    ];
    model.responses.lock().unwrap().extend([
        Ok(reply(vec![AssistantContent::ToolCall(call("next", "ping", json!({})))]).with_raw(
            json!({"tokenUsage":{"modelContextWindow":258400,"last":{"totalTokens":50000},"total":{"totalTokens":900000}}}))),
        Ok(reply(vec![AssistantContent::text("done")])),
    ]);
    assert_eq!(
        run(&model, &route, &host, Session::new(messages), false, "")
            .await
            .unwrap(),
        "done"
    );
    assert_eq!(model.requests.lock().unwrap().len(), 2);
    let events = host.events.lock().unwrap();
    assert!(
        !events
            .iter()
            .any(|(kind, _)| kind == "session/compaction" || kind == "compaction/start")
    );
    let windows: Vec<_> = events
        .iter()
        .filter(|(kind, _)| kind == "context/usage")
        .map(|(_, value)| value["contextWindow"].clone())
        .collect();
    assert_eq!(windows, vec![json!(null), json!(258400)]);
}

#[test]
fn deterministic_pruning_keeps_loaded_skills_and_the_observed_6843_character_result() {
    let host = TestHost::default();
    let skill = "规则".repeat(10000);
    let mut session = Session::new(vec![
        Message::System {
            content: "rules".into(),
        },
        Message::tool_result("skill", "mstudio_read_skill", skill.clone()),
        Message::tool_result("short", "read", "s".repeat(6843)),
        Message::tool_result(
            "long",
            "read",
            format!(
                "{}{}{}",
                "头".repeat(4096),
                "中".repeat(5000),
                "尾".repeat(1024)
            ),
        ),
        Message::assistant("consumed"),
        Message::user("continue"),
    ]);
    assert!(budget::prune_tool_results(&mut session, &host).unwrap());
    assert_eq!(
        session.messages[1],
        Message::tool_result("skill", "mstudio_read_skill", skill)
    );
    assert_eq!(
        session.messages[2],
        Message::tool_result("short", "read", "s".repeat(6843))
    );
    let text = serde_json::to_string(&session.messages[3]).unwrap();
    assert!(text.contains(&"头".repeat(4096)));
    assert!(text.contains(&"尾".repeat(1024)));
    assert!(!text.contains('中'));
    assert!(!budget::prune_tool_results(&mut session, &host).unwrap());
}
