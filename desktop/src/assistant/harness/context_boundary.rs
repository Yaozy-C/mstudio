//! Derive a continuing surface and admit changed host context at the boundary.
use super::session::project_snapshot_text;
use rig_core::message::Message;

pub fn refresh_snapshot(history: &[Message], additions: &mut Vec<Message>) {
    let Some(index) = additions
        .iter()
        .position(|m| project_snapshot_text(m).is_some())
    else {
        return;
    };
    let last = history.iter().rev().find_map(project_snapshot_text);
    // As in DSH RuntimeContextProjection, unchanged context emits nothing;
    // changed context is appended instead of rewriting historical evidence.
    if last == project_snapshot_text(&additions[index]) {
        additions.remove(index);
    }
}

pub fn continue_messages(history: Vec<Message>, fresh: &[Message], resume: bool) -> Vec<Message> {
    let mut messages = match fresh.first() {
        Some(Message::System { content }) => super::handoff::messages(content.clone(), &history),
        _ => history,
    };
    // Fresh input is system + optional fallback chat + snapshot + current user.
    // Only the snapshot and new input enter an already-restored session.
    let mut additions = fresh.iter().rev().take(2).cloned().collect::<Vec<_>>();
    additions.reverse();
    if resume {
        additions.pop(); // The interrupted input is already in the durable log.
    }
    refresh_snapshot(&messages, &mut additions);
    messages.extend(additions);
    messages
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assistant::harness::session, database::Store};
    use serde_json::json;
    fn snapshot(rev: u32) -> Message {
        super::super::context_source::snapshot(json!({"revision":rev}))
    }
    #[test]
    fn boundary_appends_changed_snapshots_and_roundtrips_through_journal() {
        let mut history = vec![
            Message::System {
                content: "rules".into(),
            },
            snapshot(1),
            Message::user("不能改变镜头动作"),
            Message::assistant("已保存"),
            snapshot(2),
        ];
        let mut additions = vec![snapshot(3), Message::user("改为1.5秒")];
        refresh_snapshot(&history, &mut additions);
        history.extend(additions);
        assert_eq!(
            history
                .iter()
                .filter(|m| project_snapshot_text(m).is_some())
                .count(),
            3
        );
        assert!(
            serde_json::to_string(&history)
                .unwrap()
                .contains("不能改变镜头动作")
        );
        let dir = std::env::temp_dir().join(format!("boundary-{}", mstudio::media::id()));
        let store = Store::open(dir.clone()).unwrap();
        store
            .db
            .lock()
            .unwrap()
            .execute("INSERT INTO projects VALUES('p','p','{}',0)", [])
            .unwrap();
        let binding = json!({"agentId":"coordinator","model":"m"});
        session::start(&store, "p", "t", binding.clone(), &history).unwrap();
        let restored = session::restore(&store, "p", "t", &binding)
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            serde_json::to_value(&history).unwrap()
        );
        let before = serde_json::to_value(&history).unwrap();
        let mut next = vec![snapshot(3)];
        refresh_snapshot(&history, &mut next);
        assert!(next.is_empty());
        assert_eq!(serde_json::to_value(&history).unwrap(), before);
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
