//! Archived generation work remains durable, but only facts enter default context.
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub fn tasks(doc: &Value) -> Vec<&Value> {
    doc["production"]["drafts"]
        .as_object()
        .into_iter()
        .flat_map(|m| m.values())
        .filter(|t| t["hiddenFromList"] == true)
        .collect()
}
pub fn summary(doc: &Value, task: &Value) -> Value {
    let ids: BTreeSet<_> = task["resultAssetIds"]
        .as_array()
        .into_iter()
        .flatten()
        .chain(std::iter::once(&task["resultAssetId"]))
        .filter_map(Value::as_str)
        .collect();
    let usable: Vec<_> = ids
        .iter()
        .filter(|id| {
            doc["assets"]
                .as_array()
                .is_some_and(|a| a.iter().any(|a| a["id"] == **id && a["discarded"] != true))
        })
        .collect();
    json!({"id":task["key"],"kind":task["kind"],"summary":task["instruction"].as_str().unwrap_or("").chars().take(120).collect::<String>(),"targetNodeId":task["targetNodeId"],"status":task["status"],"archived":true,"resultAssetIds":usable,"unavailableResultCount":ids.len()-usable.len(),"readWith":{"tool":"mstudio_read_generation","taskKey":task["key"]}})
}
pub fn snapshot(doc: &Value) -> Value {
    let mut archived = tasks(doc);
    archived.sort_by_key(|t| std::cmp::Reverse(t["createdAt"].as_i64().unwrap_or(0)));
    json!({"total":archived.len(),"items":archived.iter().take(20).map(|t|summary(doc,t)).collect::<Vec<_>>(),"more":"Use mstudio_read_generation to page compact history; taskKey reads full details on demand."})
}
// Resume and fork checkpoints containing old tool transcripts must not reintroduce
// newly archived work. New turns use bounded chat history plus fresh summaries.
pub fn revision(doc: &Value) -> Value {
    json!({"archived":tasks(doc).iter().map(|t|t["key"].clone()).collect::<Vec<_>>(),"discarded":doc["assets"].as_array().into_iter().flatten().filter(|a|a["discarded"]==true).map(|a|a["id"].clone()).collect::<Vec<_>>()})
}
pub fn archived_turns(doc: &Value) -> Vec<String> {
    let all: Vec<_> = doc["production"]["drafts"]
        .as_object()
        .into_iter()
        .flat_map(|m| m.values())
        .collect();
    let candidates: BTreeSet<_> = tasks(doc)
        .iter()
        .filter_map(|t| t["turnId"].as_str())
        .collect();
    candidates
        .into_iter()
        .filter(|turn| {
            !all.iter()
                .any(|t| t["turnId"] == *turn && t["hiddenFromList"] != true)
        })
        .map(str::to_owned)
        .collect()
}
pub fn history_summaries(doc: &Value) -> Value {
    let turns: BTreeSet<_> = tasks(doc)
        .iter()
        .filter_map(|t| t["turnId"].as_str())
        .collect();
    let all = doc["production"]["drafts"].as_object();
    let summaries: serde_json::Map<String, Value> = turns.into_iter().map(|turn| {
        let facts: Vec<_> = all.into_iter().flat_map(|m|m.values()).filter(|t|t["turnId"]==turn).map(|t|{
            let mut fact = summary(doc,t); fact["archived"] = json!(t["hiddenFromList"]==true); fact
        }).collect();
        (turn.into(),json!({"generationHistory":facts,"details":"Read exact task IDs only when needed; archived work is not pending."}))
    }).collect();
    Value::Object(summaries)
}
pub fn scrub(value: &mut Value, prompts: &[Value]) {
    match value {
        Value::String(text) => {
            for prompt in prompts
                .iter()
                .filter_map(Value::as_str)
                .filter(|p| !p.is_empty())
            {
                *text = text.replace(
                    prompt,
                    "[Archived task prompt omitted; read task details by ID when needed.]",
                );
            }
        }
        Value::Array(items) => items.iter_mut().for_each(|v| scrub(v, prompts)),
        Value::Object(fields) => fields.values_mut().for_each(|v| scrub(v, prompts)),
        _ => {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn history_has_only_facts_and_mixed_turns_remain_available() {
        let doc = json!({"assets":[{"id":"good"},{"id":"bad","discarded":true}],"production":{"drafts":{
            "old":{"key":"old","hiddenFromList":true,"turnId":"old-turn","prompt":"secret old prompt","inputs":[{"purpose":"long purpose"}],"parameters":{"width":1024},"resultAssetIds":["good","bad"]},
            "mixed":{"key":"mixed","hiddenFromList":true,"turnId":"mixed"},
            "active":{"key":"active","turnId":"mixed"}}}});
        let brief = snapshot(&doc).to_string();
        assert!(!brief.contains("secret") && !brief.contains("purpose") && !brief.contains("1024"));
        assert!(brief.contains("good") && !brief.contains("\"bad\""));
        assert_eq!(archived_turns(&doc), vec!["old-turn"]);
        let mut history = json!([{"text":"Keep unrelated decision. secret old prompt"}]);
        scrub(&mut history, &[json!("secret old prompt")]);
        assert!(history.to_string().contains("Keep unrelated decision"));
        assert!(!history.to_string().contains("secret old prompt"));
        assert_eq!(
            doc["production"]["drafts"]["old"]["prompt"],
            "secret old prompt"
        );
        let snapshot = crate::assistant::context::project_snapshot(&doc, None);
        let messages = crate::assistant::context::assemble(&[], json!("current request"), snapshot)
            .unwrap()
            .to_string();
        assert!(!messages.contains("secret old prompt"));
    }
}
