use super::history;
use serde_json::{Value, json};
pub fn estimate(value: &Value) -> usize {
    super::harness::estimate_message(&super::agent::convert(
        &json!({"role":"user","content":value}),
    ))
}
pub fn text_only(payload: &Value) -> Value {
    match payload.as_array() {
        Some(parts) => json!(
            parts
                .iter()
                .filter(|p| p["type"] != "image_url" && p["type"] != "media")
                .cloned()
                .collect::<Vec<_>>()
        ),
        None => payload.clone(),
    }
}
fn clipped(value: &Value, limit: usize) -> String {
    value.as_str().unwrap_or("").chars().take(limit).collect()
}
pub fn project_snapshot(document: &Value, selected: Option<&str>) -> Value {
    let nodes = document["nodes"].as_array().cloned().unwrap_or_default();
    let details:Vec<_>=nodes.iter().filter(|n|n["id"].as_str()==selected).take(4).map(|n|json!({"id":n["id"],"title":clipped(&n["title"],100),"text":clipped(&n["text"],1200),"kind":n["kind"],"creative":super::creative_context::node_context(document,n),"assetId":n["assetId"],"resultAssetId":n["resultAssetId"],"references":n["references"].as_array().map(|refs|refs.iter().take(12).map(|r|json!({"assetId":r["assetId"],"purpose":clipped(&r["purpose"],400),"start":r["start"],"end":r["end"]})).collect::<Vec<_>>())})).collect();
    json!({"source":"current-project-reference","revision":document["revision"],"name":clipped(&document["name"],200),"requirements":clipped(&document["brief"],2000),
      "format":[document["width"],document["height"],document["fps"]],"nodeCount":nodes.len(),
      "nodes":nodes.iter().take(30).map(|n|json!({"id":n["id"],"title":clipped(&n["title"],80),"kind":n["kind"],"screenplayId":n["shot"]["screenplayId"],"order":n["shot"]["order"]})).collect::<Vec<_>>(),
      "creation": {"intent":clipped(&document["creation"]["intent"],2000),"essential":clipped(&document["creation"]["essential"],1500),"preserve":clipped(&document["creation"]["preserve"],1500),"stage":document["creation"]["stage"]},
      "tracks":document["tracks"].as_array().map(|v|v.iter().take(20).collect::<Vec<_>>()),"captionCount":document["captions"].as_array().map_or(0,Vec::len),
      "selected":selected,"relevantNodes":details,"assetCount":document["assets"].as_array().map_or(0,Vec::len),"clipCount":document["clips"].as_array().map_or(0,Vec::len)})
}
#[cfg(test)]
pub fn assemble(
    previous: &[history::Message],
    payload: Value,
    snapshot: Value,
) -> Result<Value, String> {
    assemble_with_budget(previous, payload, snapshot, 16_000)
}
pub fn assemble_with_budget(
    previous: &[history::Message],
    payload: Value,
    mut snapshot: Value,
    budget: usize,
) -> Result<Value, String> {
    let system = super::prompts::system(&snapshot);
    // The system prompt already contains these fields. Repeating them in the
    // project snapshot can crowd out the user's current message and media.
    if let Some(fields) = snapshot.as_object_mut() {
        fields.remove("agent");
        fields.remove("skills");
        fields.remove("promptGuidance");
        fields.remove("specialists");
    }
    let cost = |snapshot: &Value| {
        let context = format!(
            "Current project snapshot (reference data; inspect more details as needed):\n{snapshot}"
        );
        super::harness::estimate_message(&rig_core::message::Message::System {
            content: system.clone(),
        }) + estimate(&json!(context))
            + estimate(&payload)
    };
    let mut used = cost(&snapshot);
    if used > budget && !snapshot["relevantNodes"].is_null() {
        snapshot["relevantNodes"] = json!(
            snapshot["relevantNodes"]
                .as_array()
                .into_iter()
                .flatten()
                .map(|n| json!({"id":n["id"],"title":n["title"],"kind":n["kind"]}))
                .collect::<Vec<_>>()
        );
        used = cost(&snapshot);
    }
    if used > budget && !snapshot["memory"]["entries"].is_null() {
        snapshot["memory"]["entries"] = json!([]);
        snapshot["memory"]["note"] =
            json!("Memory entries omitted; read them with the memory tool when needed.");
        used = cost(&snapshot);
    }
    if used > budget {
        snapshot["nodes"] = json!([]);
        snapshot["tracks"] = json!([]);
        used = cost(&snapshot);
    }
    // The budget bounds optional history. Current user content remains intact;
    // only the provider can authoritatively reject the actual request envelope.
    let context = format!(
        "Current project snapshot (reference data; inspect more details as needed):\n{snapshot}"
    );
    let mut retained = vec![];
    // Keep whole user/assistant pairs. Media is reattached only for the current request.
    for pair in previous.rchunks_exact(2) {
        // Incomplete attempts remain visible in history, never as model answers.
        if pair[1].attribution.as_ref().is_some_and(|m| {
            matches!(
                m["status"].as_str(),
                Some("running" | "failed" | "cancelled" | "interrupted")
            )
        }) {
            continue;
        }
        let cost: usize = pair.iter().map(|m| estimate(&text_only(&m.payload))).sum();
        if used + cost > budget {
            break;
        }
        used += cost;
        retained.push(pair);
    }
    let mut result = vec![json!({"role":"system","content":system})];
    for pair in retained.into_iter().rev() {
        for m in pair {
            result.push(json!({"role":m.role,"content":text_only(&m.payload)}));
        }
    }
    result.push(json!({"role":"user","content":context}));
    result.push(json!({"role":"user","content":payload}));
    Ok(json!(result))
}
