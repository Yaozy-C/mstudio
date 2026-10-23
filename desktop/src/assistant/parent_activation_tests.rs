use super::*;
use crate::assistant::history;
#[test]
fn idle_completion_wakes_once_and_running_completion_is_steered() {
    let mut d = Delivery { phase: Phase::Idle };
    assert!(d.notify());
    assert!(!d.notify());
    assert_eq!(d.phase, Phase::Running);
    // Already consumed by the running parent's next step: no extra turn.
    assert!(!d.idle(false));
    assert_eq!(d.phase, Phase::Idle);
}
#[test]
fn completion_during_finalization_is_not_lost() {
    let mut d = Delivery {
        phase: Phase::Running,
    };
    assert!(!d.notify());
    assert!(d.idle(true));
    assert_eq!(d.phase, Phase::Running);
}
#[test]
fn closing_parent_retains_notice_without_waking() {
    let mut d = Delivery {
        phase: Phase::Closing,
    };
    assert!(!d.notify());
    assert!(!d.idle(true));
    assert_eq!(d.phase, Phase::Closing);
}
#[test]
fn idle_followup_reuses_saved_request_and_never_reopens_cancelled_response() {
    let (root, store, _) = super::super::attachment_tests::fixture();
    let request = json!({"refs":[],"production":{"projectId":"p","instruction":"Create a 15s ad"}});
    let meta = json!({"agentId":"coordinator","turnId":"parent","request":request});
    history::begin(&store, "p", "parent", &request, "model", &meta).unwrap();
    history::finish(
        &store,
        "p",
        "parent",
        "Creative task running",
        "waiting",
        None,
    )
    .unwrap();
    reopen_response(&store, "p", "parent").unwrap();
    assert!(reopen_response(&store, "p", "parent").is_err());
    history::finish(&store, "p", "parent", "Script saved", "completed", None).unwrap();
    let history = history::read(&store, "p").unwrap();
    assert_eq!(history.len(), 2);
    assert_eq!(history[0].content, "Create a 15s ad");
    assert_eq!(history[1].content, "Script saved");
    assert_eq!(history[0].attribution.as_ref().unwrap()["request"], request);
    reopen_response(&store, "p", "parent").unwrap();
    history::finish(&store, "p", "parent", "Stopped", "cancelled", None).unwrap();
    assert!(reopen_response(&store, "p", "parent").is_err());
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
