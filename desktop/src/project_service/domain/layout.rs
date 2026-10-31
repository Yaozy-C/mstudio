use super::*;
pub(super) fn key(parts: &[&str]) -> String {
    parts
        .iter()
        .map(|s| {
            let mut out = String::new();
            for b in s.bytes() {
                if b.is_ascii_alphanumeric() || b"-_.!~*'()".contains(&b) {
                    out.push(b as char)
                } else {
                    out.push_str(&format!("%{b:02X}"));
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join(":")
}
pub(super) fn size(a: &Value) -> Result<(f64, f64)> {
    let w = num(&a["width"], 0.);
    let h = num(&a["height"], 0.);
    ensure!(w > 0. && h > 0., "素材画面尺寸无效");
    let width = if a["kind"] == "video" { 260. } else { 250. };
    Ok((width, 54. + (width - 26.) * h / w))
}
pub(super) fn ensure_production(p: &mut Value, field: &str) {
    if !p["production"].is_object() {
        p["production"] = json!({});
    }
    if !p["production"][field].is_object() {
        p["production"][field] = json!({});
    }
}
fn overlaps(a: &Value, b: &Value, padding: f64) -> bool {
    let x = num(&a["x"], 0.);
    let y = num(&a["y"], 0.);
    num(&b["x"], 0.) < x + num(&a["width"], 250.) + padding
        && num(&b["x"], 0.) + num(&b["width"], 250.) + padding > x
        && num(&b["y"], 0.) < y + num(&a["height"], 350.) + padding
        && num(&b["y"], 0.) + num(&b["height"], 350.) + padding > y
}
pub(super) fn items(p: &Value) -> Result<Vec<Value>> {
    let mut out = Vec::new();
    for n in rows(&p["nodes"])
        .iter()
        .filter(|n| !n["shot"].is_object() && !matches!(text(&n["kind"]), "screenplay" | "shot"))
    {
        let a = asset(p, &n["assetId"]);
        let (w, h) = if let Some(a) = a.filter(|a| matches!(text(&a["kind"]), "image" | "video")) {
            size(a)?
        } else {
            (num(&n["width"], 260.), num(&n["height"], 310.))
        };
        let aid = a.map(|a| a["id"].clone()).unwrap_or(Value::Null);
        let kind = match a.map(|a| text(&a["kind"])) {
            Some("image") => "reference",
            Some("video") => "video",
            _ => "note",
        };
        out.push(json!({"key":key(&["node",text(&n["id"])]),"kind":kind,"nodeId":n["id"],"assetId":aid,"x":num(&n["x"],0.),"y":num(&n["y"],0.),"width":w,"height":h}));
    }
    for item in &mut out {
        if let Some(pos) = p["production"]["positions"].get(text(&item["key"])) {
            assign(item, pos);
        }
    }
    Ok(out)
}
fn obstacles(p: &Value, t: &Value) -> Result<Vec<Value>> {
    let mut out = items(p)?;
    for run in p["production"]["drafts"]
        .as_object()
        .into_iter()
        .flat_map(|m| m.values())
    {
        if run["key"] != t["key"]
            && !text(&run["turnId"]).is_empty()
            && run["position"].is_object()
            && !matches!(text(&run["status"]), "COMPLETED" | "FAILED" | "CANCELLED")
            && run["resultAssetId"].is_null()
            && rows(&run["resultAssetIds"]).is_empty()
        {
            let mut o = run["position"].clone();
            o["width"] = json!(if run["kind"] == "video" { 260 } else { 250 });
            o["height"] = json!(350);
            out.push(o);
        }
    }
    Ok(out)
}
pub(super) fn free(p: &Value, t: &Value, pos: Value, size: Option<(f64, f64)>) -> Result<Value> {
    let (w, h) = size.unwrap_or((if t["kind"] == "video" { 260. } else { 250. }, 350.));
    let obstacles = obstacles(p, t)?;
    let mut card = pos;
    card["width"] = json!(w);
    card["height"] = json!(h);
    while obstacles.iter().any(|o| overlaps(&card, o, 32.)) {
        let y = num(&card["y"], 0.) + h + 45.;
        ensure!(y.is_finite() && y > num(&card["y"], 0.), "结果位置超出范围");
        card["y"] = json!(y);
    }
    Ok(select(&card, &["x", "y"]))
}
pub(super) fn placement(p: &Value, t: &Value, canvas: &Value) -> Result<Value> {
    let items = items(p)?;
    let explicit: Vec<_> = rows(&t["inputs"])
        .iter()
        .filter(|r| matches!(text(&r["role"]), "edit" | "first-frame"))
        .collect();
    let mut candidates: Vec<_> = items
        .iter()
        .filter(|n| explicit.iter().any(|r| r["key"] == n["key"]))
        .collect();
    if candidates.is_empty() {
        candidates = items
            .iter()
            .filter(|n| {
                explicit
                    .iter()
                    .any(|r| !r["assetId"].is_null() && r["assetId"] == n["assetId"])
            })
            .collect();
    }
    if candidates.len() == 1 {
        let n = candidates[0];
        return free(
            p,
            t,
            json!({"x":num(&n["x"],0.)+num(&n["width"],0.)+32.,"y":n["y"]}),
            None,
        );
    }
    let view = canvas
        .get("view")
        .or_else(|| p["production"].get("viewport"))
        .unwrap_or(&p["viewport"]);
    let scale = num(&view["scale"], 1.);
    let valid = scale > 0.;
    let scale = if valid { scale } else { 1. };
    let vx = if valid { num(&view["x"], 0.) } else { 0. };
    let vy = if valid { num(&view["y"], 0.) } else { 0. };
    let left = (32. - vx) / scale;
    let top = (80. - vy) / scale;
    let right = (num(&canvas["size"]["width"], 900.) - 32. - vx) / scale;
    let bottom = (num(&canvas["size"]["height"], 650.) - 32. - vy) / scale;
    ensure!(
        [left, top, right, bottom].iter().all(|n| n.is_finite()),
        "结果位置超出范围"
    );
    if scale < 0.35
        && let Some(n) = items.iter().find(|n| {
            num(&n["x"], 0.) + num(&n["width"], 0.) > left
                && num(&n["x"], 0.) < right
                && num(&n["y"], 0.) + num(&n["height"], 0.) > top
                && num(&n["y"], 0.) < bottom
        })
    {
        return free(
            p,
            t,
            json!({"x":num(&n["x"],0.)+num(&n["width"],0.)+32.,"y":n["y"]}),
            None,
        );
    }
    let obs = obstacles(p, t)?;
    let w = if t["kind"] == "video" { 260. } else { 250. };
    let mut y = top;
    while y + 350. <= bottom {
        let mut x = left;
        while x + w <= right {
            let c = json!({"x":x,"y":y,"width":w,"height":350});
            if !obs.iter().any(|o| overlaps(&c, o, 32.)) {
                return Ok(select(&c, &["x", "y"]));
            }
            x += w + 32.;
        }
        y += 395.;
    }
    free(p, t, json!({"x":left,"y":top}), None)
}
