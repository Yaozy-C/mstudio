use super::{
    super::{scheduler, session::Session},
    support::*,
};
use serde_json::json;
use std::sync::atomic::Ordering;
#[tokio::test]
async fn parallel_reads_overlap_but_writes_are_barriers_and_results_stay_ordered() {
    let host = TestHost::default();
    let calls = [
        call("a", "read", json!({"delay":30})),
        call("b", "read", json!({"delay":1})),
        call("c", "write", json!({})),
        call("d", "read", json!({})),
    ];
    scheduler::execute(&host, &mut Session::new(vec![]), &calls)
        .await
        .unwrap();
    let trace = host.trace.lock().unwrap();
    let index = |event: &str| trace.iter().position(|s| s == event).unwrap();
    assert!(index("start:b") < index("end:a"));
    assert!(index("end:a") < index("start:c"));
    assert!(index("end:b") < index("start:c"));
    assert!(index("end:c") < index("start:d"));
    let events = host.events.lock().unwrap();
    let ids: Vec<_> = events
        .iter()
        .filter(|(kind, _)| kind == "tool/result")
        .map(|(_, v)| v["callId"].as_str().unwrap())
        .collect();
    assert_eq!(ids, vec!["a", "b", "c", "d"]);
}
#[tokio::test]
async fn rolling_pool_is_bounded_and_replenishes_before_slow_first_call_finishes() {
    let host = TestHost::default();
    let calls: Vec<_> = (0..9)
        .map(|i| {
            call(
                &i.to_string(),
                "read",
                json!({"delay":if i==0 {60} else {1}}),
            )
        })
        .collect();
    scheduler::execute(&host, &mut Session::new(vec![]), &calls)
        .await
        .unwrap();
    assert_eq!(host.peak.load(Ordering::SeqCst), 4);
    let trace = host.trace.lock().unwrap();
    assert!(
        trace.iter().position(|s| s == "start:4").unwrap()
            < trace.iter().position(|s| s == "end:0").unwrap()
    );
}
#[tokio::test]
async fn cancellation_drains_started_write_and_pairs_unstarted_calls() {
    let host = TestHost::default();
    let calls = [
        call("a", "write", json!({"cancel":true,"delay":20})),
        call("b", "write", json!({})),
    ];
    let mut session = Session::new(vec![]);
    scheduler::execute(&host, &mut session, &calls)
        .await
        .unwrap();
    assert_eq!(*host.trace.lock().unwrap(), vec!["start:a", "end:a"]);
    assert_eq!(session.messages.len(), 2);
    let events = host.events.lock().unwrap();
    let results: Vec<_> = events.iter().filter(|(k, _)| k == "tool/result").collect();
    assert_eq!(results[0].1["result"]["ok"], true);
    assert_eq!(results[1].1["result"]["code"], "ABORTED_BEFORE_DISPATCH");
}
#[tokio::test]
async fn invalid_or_unavailable_tools_return_errors_without_entering_the_body() {
    let host = TestHost::default();
    let calls = [
        call("a", "absent", json!({})),
        call("b", "write", json!({"delay":"bad"})),
    ];
    scheduler::execute(&host, &mut Session::new(vec![]), &calls)
        .await
        .unwrap();
    assert!(host.trace.lock().unwrap().is_empty());
    let events = host.events.lock().unwrap();
    let codes: Vec<_> = events
        .iter()
        .filter(|(k, _)| k == "tool/result")
        .map(|(_, v)| v["result"]["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes, vec!["UNKNOWN_TOOL", "INVALID_ARGS"]);
}
