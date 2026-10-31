use super::*;
fn selected(saved: &Value, requested: &Value) -> Value {
    let Some(fields) = requested.as_object() else {
        return saved.clone();
    };
    let mut out = json!({});
    for (k, v) in fields {
        out[k] = selected(&saved[k], v);
    }
    out
}
pub(super) fn receipt(
    before: &Value,
    after: &Value,
    operations: &[Value],
    execution: &Value,
) -> Value {
    let saved:Vec<_>=operations.iter().filter(|o|matches!(text(&o["op"]),"update_node"|"add_node")).map(|o|{let n=node(after,&o["id"]);let mut fields=o.clone();obj(&mut fields).remove("op");obj(&mut fields).remove("id");if fields["screenplay"].is_object(){let commands=["removeParagraphIds","paragraphOrder"];let changed=commands.iter().any(|k|fields["screenplay"].get(*k).is_some());for k in commands{obj(&mut fields["screenplay"]).remove(k);}if changed{fields["screenplay"]["script"]=json!([]);}}json!({"id":o["id"],"exists":n.is_some(),"complete":true,"values":selected(n.unwrap_or(&Value::Null),&fields)})}).collect();
    let mut ids = Vec::new();
    for c in rows(&before["clips"]).iter().chain(rows(&after["clips"])) {
        if !ids.contains(&c["id"]) {
            ids.push(c["id"].clone());
        }
    }
    let mut delta = Vec::new();
    for id in ids {
        let old = rows(&before["clips"]).iter().find(|c| c["id"] == id);
        let current = rows(&after["clips"]).iter().find(|c| c["id"] == id);
        if old.zip(current).is_some_and(|(a, b)| equal(a, b)) || old.is_none() && current.is_none()
        {
            continue;
        }
        let keys: HashSet<_> = old
            .into_iter()
            .flat_map(|c| c.as_object().into_iter().flat_map(|m| m.keys()))
            .chain(
                current
                    .into_iter()
                    .flat_map(|c| c.as_object().into_iter().flat_map(|m| m.keys())),
            )
            .collect();
        let mut values = json!({});
        for k in keys {
            let a = old.and_then(|c| c.get(k));
            let b = current.and_then(|c| c.get(k));
            if k != "id" && a != b {
                values[k] = b.cloned().unwrap_or(Value::Null);
            }
        }
        delta.push(json!({"id":id,"exists":current.is_some(),"values":values}));
    }
    let script_ids: Vec<_> = operations
        .iter()
        .filter(|o| {
            ["script", "removeParagraphIds", "paragraphOrder"]
                .iter()
                .any(|k| o["screenplay"].get(*k).is_some())
        })
        .map(|o| o["id"].clone())
        .collect();
    let scripts:Vec<_>=rows(&after["nodes"]).iter().filter(|n|n["kind"]=="screenplay"&&script_ids.contains(&n["id"])).map(|n|{let script=rows(&n["screenplay"]["script"]);let seconds=script.iter().map(|s|s["duration"].as_f64().filter(|n|*n>0.).unwrap_or(5.)).sum::<f64>();json!({"id":n["id"],"title":n["title"],"paragraphCount":script.len(),"duration":(seconds*100.).round()/100.,"paragraphIds":script.iter().map(|s|s["id"].clone()).collect::<Vec<_>>(),"message":"Structured script saved to the screenplay; existing shots and media retained."})}).collect();
    let updated: Vec<_> = operations
        .iter()
        .filter(|o| matches!(text(&o["op"]), "update_generation" | "archive_generation"))
        .map(|o| {
            after["production"]["drafts"]
                .get(text(&o["taskKey"]))
                .map(|t| {
                    let mut out = generation::outcome(t);
                    if o["op"] == "update_generation" {
                        out["prompt"] = t["prompt"].clone();
                    }
                    out
                })
                .unwrap_or(Value::Null)
        })
        .collect();
    let prefix = format!("run:{}:", text(execution));
    let regen = format!("{}:", text(execution));
    let tasks: Vec<_> = generation::tasks(after)
        .into_iter()
        .filter(|t| {
            text(&t["key"]).starts_with(&prefix)
                || text(&t["lastRegenerationId"]).starts_with(&regen)
        })
        .map(generation::outcome)
        .collect();
    let changed: Vec<_> = operations
        .iter()
        .map(|o| {
            let mut v = select(o, &["op", "id"]);
            v["fields"] = json!(
                o.as_object()
                    .into_iter()
                    .flat_map(|m| m.keys())
                    .filter(|k| !matches!(k.as_str(), "op" | "id"))
                    .collect::<Vec<_>>()
            );
            v
        })
        .collect();
    json!({"applied":true,"stage":"commit","outcome":"committed","executionId":execution,"savedValues":saved,"savedClips":{"items":delta.iter().take(12).collect::<Vec<_>>(),"total":delta.len(),"complete":delta.len()<=12},"scripts":scripts,"updatedTasks":updated,"generationTasks":tasks,"revision":after["revision"],"changed":changed,"message":"Atomically saved. Receipt is authoritative; inspect only missing details or pixels. Generation tasks have their own execution status."})
}
