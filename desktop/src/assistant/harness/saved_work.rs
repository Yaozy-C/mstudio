//! Compact committed-field inventory; prompt bodies remain in original receipts.
use serde_json::{Value, json};
pub(super) fn summary(changes: &[Value]) -> Value {
    let mut items = std::collections::BTreeMap::new();
    for change in changes {
        for saved in change["result"]["savedValues"]
            .as_array()
            .into_iter()
            .flatten()
        {
            let Some(id) = saved["id"].as_str() else {
                continue;
            };
            let entry = items
                .entry(id.to_owned())
                .or_insert_with(std::collections::BTreeSet::new);
            fields("", &saved["values"], entry);
        }
    }
    json!({"complete":true,"items":items.into_iter().map(|(id,fields)|json!({"id":id,"fields":fields})).collect::<Vec<_>>()})
}
fn fields(prefix: &str, value: &Value, out: &mut std::collections::BTreeSet<String>) {
    if let Some(object) = value.as_object() {
        for (key, value) in object {
            let path = if prefix.is_empty() {
                key.clone()
            } else {
                format!("{prefix}.{key}")
            };
            fields(&path, value, out);
        }
    } else {
        out.insert(prefix.to_owned());
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_seven_saved_prompts_survive_large_delegation_projection() {
        let changes:Vec<_>=(1..=7).map(|id|json!({"result":{"savedValues":[{"id":format!("s{id}"),"values":{"shot":{"framePrompt":"x".repeat(5000)}}}]}})).collect();
        let saved = summary(&changes);
        let call = rig_core::message::ToolCall::from_wire(
            "d",
            rig_core::message::ToolFunction::new("mstudio_delegate".try_into().unwrap(), json!({})),
        );
        let value = json!({"changes":changes,"savedWork":saved});
        let view = super::super::tool_output::project(&call, &value, Some("t"));
        assert_eq!(view["savedWork"], saved);
        assert_eq!(view["savedWork"]["items"].as_array().unwrap().len(), 7);
        assert_eq!(
            view["savedWork"]["items"][6]["fields"],
            json!(["shot.framePrompt"])
        );
    }
}

/// Read current state before a recovered delegation, independently of its prose.
pub(super) fn shot_state(doc: &Value, node: &Value) -> Value {
    let tasks: Vec<_> = doc["production"]["drafts"]
        .as_object()
        .into_iter()
        .flat_map(|tasks| tasks.values())
        .filter(|task| task["targetNodeId"] == node["id"] || task["ownerId"] == node["id"])
        .map(|task| {
            json!({"id":task["key"],"kind":task["kind"],"status":task["status"],
            "resultAssetIds":task["resultAssetIds"],"resultAssetId":task["resultAssetId"],
            "readWith":{"tool":"mstudio_read_generation","arguments":{"taskKey":task["key"]}}})
        })
        .collect();
    json!({"framePromptSaved":node["shot"]["framePrompt"].as_str().is_some_and(|s|!s.trim().is_empty()),
        "videoPromptSaved":node["shot"]["prompt"].as_str().is_some_and(|s|!s.trim().is_empty()),
        "generationTasks":tasks,"guidance":"Use matching generation task IDs and actual result assets. Do not read old prompts unless the user explicitly requests a prompt edit, unchanged retry or generation investigation. Wait for running work; use existing results when complete. Create only missing work; an explicit correction uses the existing task's regeneration tool."})
}

#[cfg(test)]
#[test]
fn recovery_snapshot_includes_saved_prompt_and_only_its_existing_tasks() {
    let node = json!({"id":"s1","shot":{"framePrompt":"saved board","prompt":"saved motion"}});
    let doc = json!({"production":{"drafts":{
        "a":{"key":"a","targetNodeId":"s1","kind":"image","status":"IN_PROGRESS"},
        "b":{"key":"b","targetNodeId":"s1","kind":"image","status":"COMPLETED","resultAssetIds":["image"]},
        "c":{"key":"c","targetNodeId":"s2","kind":"image","status":"READY"}
    }}});
    let state = shot_state(&doc, &node);
    assert_eq!(state["framePromptSaved"], true);
    assert_eq!(state["videoPromptSaved"], true);
    assert_eq!(state["generationTasks"].as_array().unwrap().len(), 2);
    assert_eq!(
        state["generationTasks"][1]["resultAssetIds"],
        json!(["image"])
    );
}
