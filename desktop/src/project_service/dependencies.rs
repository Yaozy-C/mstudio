//! Read dependencies of edits are checked before evaluating the domain batch.
use serde_json::Value;
use std::collections::BTreeSet;

pub fn required(before: &Value, args: &Value) -> BTreeSet<String> {
    let mut dependencies = BTreeSet::new();
    let mut created = BTreeSet::new();
    for op in args["operations"].as_array().into_iter().flatten() {
        if matches!(op["op"].as_str(), Some("add_node" | "update_node")) {
            let existing = before["nodes"]
                .as_array()
                .into_iter()
                .flatten()
                .find(|node| node["id"] == op["id"]);
            // Local edits (frame links, image prompts, timing, text) do not
            // consume screenplay content. Creating/rebinding a shot does; video
            // prompt writes also derive a promptBasis from the current script.
            let uses_script = existing.is_none()
                || ["screenplayId", "scriptId"].iter().any(|key| {
                    op["shot"].get(*key).is_some_and(|value| {
                        Some(value) != existing.and_then(|node| node["shot"].get(*key))
                    })
                })
                || op["shot"].get("prompt").is_some();
            let screenplay = uses_script
                .then(|| {
                    op["shot"]
                        .get("screenplayId")
                        .or_else(|| existing.and_then(|node| node["shot"].get("screenplayId")))
                        .and_then(Value::as_str)
                })
                .flatten();
            if let Some(id) = screenplay.filter(|id| !created.contains(*id)) {
                dependencies.insert(format!("node:{id}"));
            }
        }
        // A screenplay created earlier in this atomic batch has no prior snapshot.
        if op["op"] == "add_node"
            && op["kind"] == "screenplay"
            && let Some(id) = op["id"].as_str()
            && !before["nodes"]
                .as_array()
                .into_iter()
                .flatten()
                .any(|n| n["id"] == id)
        {
            created.insert(id.to_owned());
        }
    }
    dependencies
}
