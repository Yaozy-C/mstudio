//! Refresh host-owned state only at turn boundaries; session/start journals the result.
use super::session::project_snapshot_text;
use rig_core::message::Message;

pub fn refresh_snapshot(history: &mut Vec<Message>, additions: &mut Vec<Message>) {
    let Some(index) = additions
        .iter()
        .position(|m| project_snapshot_text(m).is_some())
    else {
        return;
    };
    let fresh = additions.remove(index);
    let old: Vec<_> = history
        .iter()
        .enumerate()
        .filter(|(_, m)| project_snapshot_text(m).is_some())
        .map(|(i, _)| i)
        .collect();
    let Some(&keep) = old.last() else {
        additions.insert(index, fresh);
        return;
    };
    // Identical snapshots keep their exact message and position (cache-friendly).
    if project_snapshot_text(&history[keep]) != project_snapshot_text(&fresh) {
        history[keep] = fresh;
    }
    for &index in old[..old.len() - 1].iter().rev() {
        history.remove(index);
    }
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
    fn boundary_replaces_only_snapshots_and_roundtrips_through_journal() {
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
        refresh_snapshot(&mut history, &mut additions);
        history.extend(additions);
        assert_eq!(
            history
                .iter()
                .filter(|m| project_snapshot_text(m).is_some())
                .count(),
            1
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
        refresh_snapshot(&mut history, &mut next);
        assert!(next.is_empty());
        assert_eq!(serde_json::to_value(&history).unwrap(), before);
        drop(store);
        std::fs::remove_dir_all(dir).unwrap();
    }
}
