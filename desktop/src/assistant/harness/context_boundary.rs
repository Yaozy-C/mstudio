//! Refresh host-owned state only at turn boundaries; session/start journals the result.
use super::session::project_snapshot_text;
use rig_core::message::{AssistantContent, Message, ToolResultContent, UserContent};
use std::collections::HashSet;

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

/// Memory is persisted separately and recalled in the new snapshot. Retire only
/// completed pairs when starting a new turn, never during retries or resume.
pub fn retire_memory_calls(history: &mut Vec<Message>) {
    let completed: HashSet<_> = history
        .iter()
        .filter_map(|m| match m {
            Message::User { content } => Some(content.iter().filter_map(|part| match part {
                UserContent::ToolResult(result)
                    if result.content.iter().any(|part| {
                        let ToolResultContent::Text(text) = part else {
                            return false;
                        };
                        serde_json::from_str::<serde_json::Value>(&text.text)
                            .is_ok_and(|value| value.is_object() && value.get("error").is_none())
                    }) =>
                {
                    Some(result.call.clone())
                }
                _ => None,
            })),
            _ => None,
        })
        .flatten()
        .collect();
    let retired: HashSet<_> = history
        .iter()
        .filter_map(|m| match m {
            Message::Assistant { content, .. } => {
                Some(content.iter().filter_map(|part| match part {
                    AssistantContent::ToolCall(call)
                        if call.function.name == "mstudio_memory"
                            && completed.contains(&call.id) =>
                    {
                        Some(call.id.clone())
                    }
                    _ => None,
                }))
            }
            _ => None,
        })
        .flatten()
        .collect();
    history.retain_mut(|message| match message {
        Message::Assistant { content, .. } => {
            content.retain(|part| !matches!(part, AssistantContent::ToolCall(call) if retired.contains(&call.id)));
            !content.is_empty()
        }
        Message::User { content } => {
            content.retain(|part| !matches!(part, UserContent::ToolResult(result) if retired.contains(&result.call)));
            !content.is_empty()
        }
        Message::System { .. } => true,
    });
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assistant::harness::session, database::Store};
    use serde_json::json;
    fn snapshot(rev: u32) -> Message {
        Message::user(format!(
            "当前工程快照（参考数据；更多内容请按需 inspect）：\n{{\"revision\":{rev}}}"
        ))
    }
    #[test]
    fn completed_memory_pairs_are_retired_without_touching_other_calls_or_pending_work() {
        use rig_core::message::{ToolCall, ToolFunction};
        let call = |id: &str, name: &str| {
            ToolCall::from_wire(
                id,
                ToolFunction {
                    name: name.into(),
                    arguments: json!({}),
                },
            )
        };
        let memory = call("memory", "mstudio_memory");
        let inspect = call("inspect", "mstudio_inspect");
        let pending = call("pending", "mstudio_memory");
        let memory_result = session::result_message(&memory, &json!({"memoryRevision":1}));
        let inspect_result = session::result_message(&inspect, &json!({"revision":2}));
        let Message::User {
            content: memory_content,
        } = memory_result
        else {
            unreachable!()
        };
        let Message::User {
            content: inspect_content,
        } = inspect_result.clone()
        else {
            unreachable!()
        };
        let mixed_result = Message::User {
            content: [memory_content, inspect_content].concat(),
        };
        let pending_message = Message::Assistant {
            id: None,
            content: vec![AssistantContent::ToolCall(pending)],
        };
        let original = vec![
            snapshot(2),
            Message::user("以后文案保持克制"),
            Message::Assistant {
                id: None,
                content: vec![
                    AssistantContent::ToolCall(memory),
                    AssistantContent::ToolCall(inspect.clone()),
                ],
            },
            mixed_result,
            Message::assistant("已记住"),
            pending_message.clone(),
        ];
        let mut projected = original.clone();
        retire_memory_calls(&mut projected);
        assert_eq!(
            projected,
            vec![
                snapshot(2),
                Message::user("以后文案保持克制"),
                Message::Assistant {
                    id: None,
                    content: vec![AssistantContent::ToolCall(inspect)],
                },
                inspect_result,
                Message::assistant("已记住"),
                pending_message
            ]
        );
        let mut repaired = vec![projected.last().unwrap().clone()];
        session::repair_pending(&mut repaired);
        let unknown = repaired.clone();
        retire_memory_calls(&mut repaired);
        assert_eq!(repaired, unknown);
        let once = projected.clone();
        retire_memory_calls(&mut projected);
        assert_eq!(projected, once);
        assert_eq!(
            serde_json::to_string(&original)
                .unwrap()
                .matches("mstudio_memory")
                .count(),
            3
        );
        let mut pair = vec![original[2].clone(), original[3].clone()];
        if let Message::Assistant { content, .. } = &mut pair[0] {
            content.truncate(1);
        }
        if let Message::User { content } = &mut pair[1] {
            content.truncate(1);
        }
        retire_memory_calls(&mut pair);
        assert!(pair.is_empty());
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
            Message::user("当前工程参考数据：{\"revision\":2}"),
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

#[cfg(test)]
mod language_upgrade_tests {
    use super::*;
    #[test]
    fn english_snapshot_replaces_legacy_snapshot_without_dropping_user_content() {
        let user = Message::user("保持中文对白和品牌名称");
        let mut history = vec![
            Message::user("当前工程快照（参考数据；更多内容请按需 inspect）：\n{\"revision\":1}"),
            user.clone(),
        ];
        let fresh = Message::user(
            "Current project snapshot (reference data; inspect more details as needed):\n{\"revision\":2}",
        );
        let mut additions = vec![fresh.clone()];
        refresh_snapshot(&mut history, &mut additions);
        assert!(additions.is_empty());
        assert_eq!(
            serde_json::to_value(&history).unwrap(),
            serde_json::to_value(vec![fresh, user]).unwrap()
        );
    }
}
