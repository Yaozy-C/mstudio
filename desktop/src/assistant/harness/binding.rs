use serde_json::Value;

// Profile edits do not invalidate committed history. The caller replaces the system
// prompt and assembles tools from the latest saved profile; old calls are never replayed.
pub(super) fn compatible(saved: &Value, current: &Value) -> bool {
    let (Some(mut saved), Some(mut current)) =
        (saved.as_object().cloned(), current.as_object().cloned())
    else {
        return false;
    };
    for field in ["revision", "tools"] {
        saved.remove(field);
        current.remove(field);
    }
    saved == current
}

#[cfg(test)]
mod tests {
    use crate::{assistant::harness::session, database::Store};
    use rig_core::message::Message;
    use serde_json::json;

    #[test]
    fn retry_restores_history_after_profile_edits_but_not_route_changes() {
        let root =
            std::env::temp_dir().join(format!("mstudio-profile-refresh-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        store
            .db
            .lock()
            .unwrap()
            .execute("INSERT INTO projects VALUES('p','test','{}',0)", [])
            .unwrap();
        let old = json!({"provider":"openai","endpoint":"local","model":"m","agentId":"editor","revision":1,"tools":["edit"]});
        let mut current = old.clone();
        current["revision"] = json!(2);
        current["tools"] = json!(["inspect"]);
        session::start(
            &store,
            "p",
            "turn",
            old,
            &[Message::user("keep original task")],
        )
        .unwrap();
        let restored = session::restore(&store, "p", "turn", &current)
            .unwrap()
            .unwrap();
        assert_eq!(
            serde_json::to_value(restored).unwrap(),
            serde_json::to_value(vec![Message::user("keep original task")]).unwrap()
        );
        for field in ["provider", "endpoint", "model", "agentId"] {
            let mut changed = current.clone();
            changed[field] = json!("different");
            assert!(session::restore(&store, "p", "turn", &changed).is_err());
        }
        assert!(
            session::restore(&store, "other-project", "turn", &current)
                .unwrap()
                .is_none()
        );
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
