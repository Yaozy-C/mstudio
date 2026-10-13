//! Host provenance survives journaling but is removed from the provider wire.
//! User-authored text, including an identical heading, never acquires ownership.
use rig_core::message::{AssistantContent, Message, UserContent};
use serde_json::{Value, json};
pub const KEY: &str = "mstudio.context";
pub fn message(text: String, metadata: Value) -> Message {
    let AssistantContent::Text(text) =
        AssistantContent::text(text).with_native(json!({KEY:metadata}))
    else {
        unreachable!()
    };
    Message::User {
        content: vec![UserContent::Text(text)],
    }
}
pub fn snapshot(mut snapshot: Value) -> Message {
    let metadata = metadata(&mut snapshot);
    message(
        format!("Current project reference data:{snapshot}"),
        metadata,
    )
}
pub fn metadata(snapshot: &mut Value) -> Value {
    if let Some(fields) = snapshot.as_object_mut() {
        fields.remove("_historyGeneration");
        fields.remove("_archivedPrompts");
        fields.remove("_archivedTurns");
    }
    let mut versions = snapshot
        .as_object_mut()
        .and_then(|s| s.remove("_observations"))
        .unwrap_or(json!({}));
    let visible = crate::project_service::observations::read_targets(snapshot);
    if let Some(map) = versions.as_object_mut() {
        map.retain(|key, _| visible.contains(key));
    }
    json!({"kind":"project","projectId":snapshot["projectId"],"versions":versions})
}
pub fn source(message: &Message) -> Option<(&str, &Value)> {
    let Message::User { content } = message else {
        return None;
    };
    content.iter().find_map(|part| {
        let UserContent::Text(text) = part else {
            return None;
        };
        let metadata = text.native.as_ref()?.item.get(KEY)?;
        (metadata["kind"] == "project").then_some((text.text.as_str(), metadata))
    })
}
pub fn wire(messages: &mut [Message]) {
    for message in messages {
        if let Message::User { content } = message {
            for part in content {
                if let UserContent::Text(text) = part
                    && text
                        .native
                        .as_ref()
                        .is_some_and(|n| n.item.get(KEY).is_some())
                {
                    text.native = None;
                }
            }
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn user_text_cannot_replace_host_context_and_metadata_is_not_provider_input() {
        let text = "Current project reference data: user quotation";
        assert!(source(&Message::user(text)).is_none());
        let mut messages = vec![message(
            text.into(),
            json!({"kind":"project","projectId":"p","versions":{"node:a":"v"}}),
        )];
        let encoded = serde_json::to_value(&messages).unwrap();
        let restored: Vec<Message> = serde_json::from_value(encoded).unwrap();
        assert_eq!(source(&restored[0]).unwrap().1["projectId"], "p");
        wire(&mut messages);
        assert!(source(&messages[0]).is_none());
        assert_eq!(messages[0], Message::user(text));
    }
}
