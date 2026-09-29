use super::{
    super::{Host, session::Session},
    support::*,
};
use rig_core::message::Message;
use serde_json::json;
#[test]
fn meter_anchors_usage_and_counts_signed_growth_without_double_output() {
    use super::super::budget;
    let host = TestHost::default();
    let route = profile();
    let mut session = Session::new(vec![
        Message::user("中文".repeat(100)),
        Message::assistant("answer"),
    ]);
    let raw = budget::raw_pressure(&session, &host);
    budget::anchor(&mut session, &route, &host, 2000, 500);
    assert_eq!(budget::pressure(&session, &route, &host), 2500);
    session.messages.push(Message::user("new"));
    assert_eq!(
        budget::pressure(&session, &route, &host),
        2500 + budget::raw_pressure(&session, &host) - raw
    );
    let anchor = session.usage_anchor.clone();
    session
        .replace_range(&host, 0, 1, Message::user("摘要"))
        .unwrap();
    assert_eq!(session.usage_anchor, anchor);
    assert!(budget::pressure(&session, &route, &host) < 2500);
    let mut changed = route.clone();
    changed.model = "different".into();
    assert_eq!(
        budget::pressure(&session, &changed, &host),
        budget::raw_pressure(&session, &host)
    );
}

#[test]
fn meter_does_not_scale_down_heuristic_when_usage_is_smaller() {
    use super::super::budget;
    let host = TestHost::default();
    let mut session = Session::new(vec![Message::user("verbose".repeat(2000))]);
    budget::anchor(&mut session, &profile(), &host, 2, 1);
    assert_eq!(
        budget::pressure(&session, &profile(), &host),
        budget::raw_pressure(&session, &host)
    );
}

#[test]
fn cache_usage_is_adapter_aware_and_request_header_is_fixed_at_dispatch() {
    use super::super::budget;
    let host = TestHost::default();
    let mut route = profile();
    let usage = rig_core::completion::Usage {
        input_tokens: 1000,
        output_tokens: 100,
        cached_input_tokens: 700,
        cache_creation_input_tokens: 200,
        ..Default::default()
    };
    assert_eq!(budget::usage_total(&route, &usage), 1100);
    route.adapter = "anthropic-native".into();
    assert_eq!(budget::usage_total(&route, &usage), 2000);
    let mut session = Session::new(vec![
        Message::user("request"),
        Message::assistant("response"),
    ]);
    let mut dispatched = budget::header(&route, &host.definitions());
    dispatched["tools"] = json!([]);
    budget::anchor_request(&mut session, dispatched, 10000);
    // A tool catalog change during the request must not calibrate the new catalog.
    assert_eq!(
        budget::pressure(&session, &route, &host),
        budget::raw_pressure(&session, &host)
    );
}
