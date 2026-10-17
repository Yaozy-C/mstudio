use super::File;
use serde_json::json;

/// A self-contained edit request. Stored chat is display history, never model input.
pub(super) fn request(file: &File, files: &[File], prompt: &str) -> String {
    let inventory = files
        .iter()
        .filter(|file| file.kind == "model")
        .map(|model| json!({"id":model.id,"name":model.name,"description":model.description}))
        .collect::<Vec<_>>();
    json!({"currentFile":file,"modelInventory":inventory,"request":prompt}).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn edit_payload_has_only_the_current_file_inventory_and_instruction() {
        let file = File {
            id: "shot".into(),
            kind: "shot".into(),
            name: "Shot".into(),
            description: "Fixed camera".into(),
            code: "current scene code".into(),
            revision: 4,
            asset_ids: vec![],
            bindings: Default::default(),
            duration: 2.7,
            width: 720,
            height: 1280,
            fps: 24,
        };
        let model = File {
            id: "bag".into(),
            kind: "model".into(),
            name: "Bag".into(),
            description: "Named lid hinge".into(),
            code: "private model implementation".into(),
            ..file.clone()
        };
        let unrelated = File {
            id: "other-shot".into(),
            code: "other shot code".into(),
            ..file.clone()
        };
        let payload: serde_json::Value =
            serde_json::from_str(&request(&file, &[model, unrelated], "Open the lid")).unwrap();
        assert_eq!(payload.as_object().unwrap().len(), 3);
        assert_eq!(payload["request"], "Open the lid");
        assert_eq!(payload["currentFile"]["code"], "current scene code");
        assert_eq!(
            payload["modelInventory"],
            json!([{"id":"bag","name":"Bag","description":"Named lid hinge"}])
        );
        assert!(payload.get("recentConversation").is_none());
        assert!(!payload.to_string().contains("private model implementation"));
        assert!(!payload.to_string().contains("other shot code"));
        let empty = File {
            code: String::new(),
            ..file
        };
        let payload: serde_json::Value =
            serde_json::from_str(&request(&empty, &[], "First scene")).unwrap();
        assert_eq!(payload["currentFile"]["code"], "");
        assert_eq!(payload["modelInventory"], json!([]));
    }
}
