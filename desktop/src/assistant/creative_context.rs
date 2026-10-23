use serde_json::{Value, json};
fn text(value: &Value, max: usize) -> String {
    value.as_str().unwrap_or("").chars().take(max).collect()
}
pub fn node_context(doc: &Value, node: &Value) -> Value {
    if node["kind"] == "screenplay" {
        let shots: Vec<_> = doc["nodes"].as_array().into_iter().flatten()
            .filter(|n| n["shot"]["screenplayId"] == node["id"])
            .map(|n| json!({"id":n["id"],"title":text(&n["title"],100),"order":n["shot"]["order"],"duration":n["shot"]["duration"]})).collect();
        let script: Vec<_> = node["screenplay"]["script"].as_array().into_iter().flatten().take(8).map(|s| json!({"id":s["id"],"duration":s["duration"],"title":text(&s["title"],100),"action":text(&s["action"],400),"onScreenText":text(&s["onScreenText"],400),"dialogue":text(&s["dialogue"],400),"sound":text(&s["sound"],200)})).collect();
        json!({"script":script,"scriptCount":node["screenplay"]["script"].as_array().map(Vec::len),"scriptMore":"mstudio_read_screenplay(id, offset, textOffset) reads full paragraphs","shots":shots.iter().take(20).collect::<Vec<_>>(),"shotCount":shots.len(),"more":"mstudio_read_project(offset) lists remaining nodes; mstudio_read_shots(ids) reads their design"})
    } else {
        json!({"scriptId":node["shot"]["scriptId"],"screenplayId":node["shot"]["screenplayId"],"order":node["shot"]["order"],"duration":node["shot"]["duration"],"dialogue":text(&node["shot"]["dialogue"],1600),"frames":node["shot"]["frames"].as_array().map(|a|a.iter().take(5).map(|f|json!({"assetId":f["assetId"],"title":f["title"]})).collect::<Vec<_>>()),"visualChanged":node["shot"]["visualChanged"],"takes":node["shot"]["takes"].as_array().map(|a|a.iter().take(5).map(|t|json!({"assetId":t["assetId"],"trimIn":t["trimIn"],"trimOut":t["trimOut"]})).collect::<Vec<_>>())})
    }
}
