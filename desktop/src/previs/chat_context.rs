use super::File;
use serde_json::json;

/// A self-contained edit request. Stored chat is display history, never model input.
pub(super) fn request(file: &File, files: &[File], prompt: &str) -> String {
    let inventory = files
        .iter()
        .filter(|model| model.kind == "model" && file.asset_ids.contains(&model.id))
        .map(|model| json!({"id":model.id,"name":model.name,"description":model.description,"path":format!("/models/{}.js",model.id)}))
        .collect::<Vec<_>>();
    json!({"currentFile":{"id":file.id,"kind":file.kind,"name":file.name,"description":file.description,"duration":file.duration,"width":file.width,"height":file.height,"fps":file.fps,"bindings":file.bindings},"modelInventory":inventory,"request":prompt,
        "editorPath":if file.kind=="shot" {"/scene.js"} else {"/model.js"}})
    .to_string()
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
            asset_ids: vec!["bag".into()],
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
        assert_eq!(payload.as_object().unwrap().len(), 4);
        assert_eq!(payload["editorPath"], "/scene.js");
        assert_eq!(payload["request"], "Open the lid");
        assert!(payload["currentFile"].get("code").is_none());
        assert_eq!(
            payload["modelInventory"],
            json!([{"id":"bag","name":"Bag","description":"Named lid hinge","path":"/models/bag.js"}])
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
        assert!(payload["currentFile"].get("code").is_none());
        assert_eq!(payload["modelInventory"], json!([]));
    }
}
