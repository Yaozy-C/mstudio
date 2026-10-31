use super::*;
use std::collections::BTreeSet;
fn same(a: Option<&Value>, b: Option<&Value>) -> bool {
    match (a, b) {
        (Some(a), Some(b)) => equal(a, b),
        _ => a == b,
    }
}
fn entities(v: &Value) -> bool {
    let r = rows(v);
    v.is_array()
        && r.iter().all(|v| v.is_object() && v["id"].is_string())
        && r.iter()
            .map(|v| text(&v["id"]))
            .collect::<HashSet<_>>()
            .len()
            == r.len()
}
fn conflict(path: &str) -> anyhow::Error {
    anyhow::anyhow!(json!({"code":"PROJECT_CONFLICT","message":path,"details":path}).to_string())
}
fn value(
    base: Option<&Value>,
    local: Option<&Value>,
    current: Option<&Value>,
    path: &str,
) -> Result<Option<Value>> {
    if same(base, local) {
        return Ok(current.cloned());
    }
    if same(base, current) || same(local, current) {
        return Ok(local.cloned());
    }
    if let (Some(Value::Object(b)), Some(Value::Object(l)), Some(Value::Object(c))) =
        (base, local, current)
    {
        let keys: BTreeSet<_> = b.keys().chain(l.keys()).chain(c.keys()).collect();
        let mut out = json!({});
        for key in keys {
            let next = if path.is_empty()
                && ["revision", "storageVersion", "previsAssets"].contains(&key.as_str())
            {
                c.get(key).cloned()
            } else {
                value(b.get(key), l.get(key), c.get(key), &format!("{path}/{key}"))?
            };
            if let Some(v) = next {
                out[key] = v;
            }
        }
        return Ok(Some(out));
    }
    if let (Some(b), Some(l), Some(c)) = (base, local, current)
        && entities(b)
        && entities(l)
        && entities(c)
    {
        let get = |v: &Value, id: &str| rows(v).iter().find(|v| text(&v["id"]) == id).cloned();
        let common: Vec<_> = rows(b)
            .iter()
            .map(|v| text(&v["id"]))
            .filter(|id| get(l, id).is_some() && get(c, id).is_some())
            .collect();
        let order = |v: &Value| {
            rows(v)
                .iter()
                .map(|v| text(&v["id"]).to_owned())
                .filter(|id| common.contains(&id.as_str()))
                .collect::<Vec<_>>()
        };
        let bo: Vec<_> = common.iter().map(|s| s.to_string()).collect();
        let lo = order(l);
        let co = order(c);
        ensure!(
            lo == bo || co == bo || lo == co,
            "{}",
            conflict(&format!("Concurrent order change at {path}"))
        );
        let (first, second) = if lo == bo { (c, l) } else { (l, c) };
        let mut ids = Vec::new();
        for v in rows(first).iter().chain(rows(second)).chain(rows(b)) {
            let id = text(&v["id"]);
            if !ids.contains(&id) {
                ids.push(id);
            }
        }
        let mut out = Vec::new();
        for id in ids {
            if let Some(v) = value(
                get(b, id).as_ref(),
                get(l, id).as_ref(),
                get(c, id).as_ref(),
                &format!("{path}/{id}"),
            )? {
                out.push(v)
            }
        }
        return Ok(Some(json!(out)));
    }
    Err(conflict(&format!(
        "Concurrent change at {}",
        if path.is_empty() { "/" } else { path }
    )))
}
pub(super) fn project(input: &Value) -> Result<Value> {
    let ids: Vec<_> = rows(&input["document"]["removedAssetIds"])
        .iter()
        .chain(rows(&input["current"]["removedAssetIds"]))
        .cloned()
        .collect();
    let b = prune(&input["base"], &ids);
    let l = prune(&input["document"], &ids);
    let c = prune(&input["current"], &ids);
    let mut document = prune(
        &value(Some(&b), Some(&l), Some(&c), "")?.context("Missing merged project")?,
        &ids,
    );
    creative::validate_order(&document)?;
    document["revision"] = json!(
        num(&input["current"]["revision"], 0.)
            + u8::from(content_changed(&input["current"], &document)) as f64
    );
    Ok(json!({"document":document}))
}
pub(super) fn prune(p: &Value, ids: &[Value]) -> Value {
    let mut ordered = Vec::new();
    let mut gone = HashSet::new();
    for id in rows(&p["removedAssetIds"]).iter().chain(ids) {
        if let Some(id) = id.as_str()
            && gone.insert(id.to_owned())
        {
            ordered.push(json!(id));
        }
    }
    if gone.is_empty() {
        return p.clone();
    }
    loop {
        let before = gone.len();
        for a in rows(&p["assets"]) {
            if gone.contains(text(&a["annotationSourceId"]))
                && gone.insert(text(&a["id"]).to_owned())
            {
                ordered.push(a["id"].clone());
            }
        }
        if before == gone.len() {
            break;
        }
    }
    let node_ids: HashSet<_> = rows(&p["nodes"])
        .iter()
        .filter(|n| gone.contains(text(&n["assetId"])))
        .map(|n| text(&n["id"]).to_owned())
        .collect();
    let clean = |t: &mut Value| {
        let old = rows(&t["inputs"]).len();
        t["inputs"] = rows(&t["inputs"])
            .iter()
            .filter(|r| {
                !gone.contains(text(&r["assetId"])) && !node_ids.contains(text(&r["nodeId"]))
            })
            .cloned()
            .collect();
        if gone.contains(text(&t["resultAssetId"])) {
            obj(t).remove("resultAssetId");
        }
        if t.get("resultAssetIds").is_some() {
            t["resultAssetIds"] = rows(&t["resultAssetIds"])
                .iter()
                .filter(|v| !gone.contains(text(v)))
                .cloned()
                .collect();
        }
        if old != rows(&t["inputs"]).len()
            && matches!(text(&t["status"]), "READY" | "AWAITING_CONFIRMATION")
        {
            t["status"] = json!("FAILED");
            t["error"] = json!("参考素材已删除，请重新选择后生成");
        }
    };
    let mut p = p.clone();
    p["removedAssetIds"] = json!(ordered);
    for field in ["assets", "clips", "captions"] {
        p[field] = rows(&p[field])
            .iter()
            .filter(|v| !gone.contains(text(&v[if field == "assets" { "id" } else { "assetId" }])))
            .cloned()
            .collect();
    }
    let mut nodes = Vec::new();
    for mut n in rows(&p["nodes"])
        .iter()
        .filter(|n| !node_ids.contains(text(&n["id"])))
        .cloned()
    {
        if gone.contains(text(&n["resultAssetId"])) {
            obj(&mut n).remove("resultAssetId");
        }
        if n.get("references").is_some() {
            n["references"] = rows(&n["references"])
                .iter()
                .filter(|r| !gone.contains(text(&r["assetId"])))
                .cloned()
                .collect();
        }
        for field in ["frames", "takes"] {
            if n["shot"].get(field).is_some() {
                let mut list: Vec<_> = rows(&n["shot"][field])
                    .iter()
                    .filter(|r| !gone.contains(text(&r["assetId"])))
                    .cloned()
                    .collect();
                if field == "takes" {
                    for take in &mut list {
                        if take["production"].is_object() {
                            clean(&mut take["production"]);
                        }
                    }
                }
                n["shot"][field] = json!(list);
            }
        }
        nodes.push(n)
    }
    p["nodes"] = json!(nodes);
    let remove_key = |key: &str| {
        let parts: Vec<_> = key.split(':').map(percent_decode).collect();
        if parts.first().is_some_and(|s| s == "node") {
            parts.get(1).is_some_and(|s| node_ids.contains(s))
        } else {
            parts.get(2).is_some_and(|s| gone.contains(s))
        }
    };
    if let Some(positions) = p
        .get_mut("production")
        .and_then(|v| v.get_mut("positions"))
        .and_then(Value::as_object_mut)
    {
        positions.retain(|k, _| !remove_key(k));
    }
    if let Some(tasks) = p
        .get_mut("production")
        .and_then(|v| v.get_mut("drafts"))
        .and_then(Value::as_object_mut)
    {
        for t in tasks.values_mut() {
            clean(t)
        }
    }
    p
}
pub(super) fn percent_decode(s: &str) -> String {
    let bytes = s.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%'
            && i + 2 < bytes.len()
            && let Ok(hex) = std::str::from_utf8(&bytes[i + 1..i + 3])
            && let Ok(b) = u8::from_str_radix(hex, 16)
        {
            out.push(b);
            i += 3;
            continue;
        }
        out.push(bytes[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}
