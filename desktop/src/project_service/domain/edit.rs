use super::*;
fn index(p: &Value, section: &str, id: &Value) -> Result<usize> {
    rows(&p[section])
        .iter()
        .position(|v| &v["id"] == id)
        .with_context(|| format!("{section} 中的对象不存在"))
}
fn remove_nodes(p: &mut Value, ids: &[Value]) {
    let mut ids = ids.to_vec();
    for n in rows(&p["nodes"]) {
        if ids.contains(&n["shot"]["screenplayId"]) {
            ids.push(n["id"].clone());
        }
    }
    array(&mut p["nodes"]).retain(|n| !ids.contains(&n["id"]));
    for c in array(&mut p["clips"]) {
        if ids.contains(&c["shotId"]) {
            obj(c).remove("shotId");
        }
    }
}
fn concept_card(input: &Value, existing: Option<&Value>) -> Result<Value> {
    let pick = |key: &str, max: usize| -> Result<String> {
        let value = input
            .get(key)
            .or_else(|| existing.and_then(|e| e.get(key)))
            .with_context(|| format!("概念缺少字段：{key}"))?;
        Ok(checked_text(value, max)?.to_owned())
    };
    let title = pick("title", 200)?;
    let one_line = pick("oneLine", 600)?;
    let optional = |key: &str, max: usize| -> Result<String> {
        input
            .get(key)
            .or_else(|| existing.and_then(|e| e.get(key)))
            .map(|v| checked_text(v, max).map(str::to_owned))
            .transpose()
            .map(|v| v.unwrap_or_default())
    };
    let truth = optional("truth", 600)?;
    let brand_idea = optional("brandIdea", 600)?;
    let participant = pick("participant", 600)?;
    let want = pick("want", 600)?;
    let ritual = pick("ritual", 600)?;
    let evidence = pick("evidence", 1200)?;
    let reframe = pick("reframe", 600)?;
    let risk = pick("risk", 600)?;
    let cost = input
        .get("cost")
        .or_else(|| existing.and_then(|e| e.get("cost")))
        .map(|v| checked_text(v, 120).map(str::to_owned))
        .transpose()?
        .unwrap_or_default();
    let status = input
        .get("status")
        .or_else(|| existing.and_then(|e| e.get("status")))
        .and_then(Value::as_str)
        .unwrap_or("rough");
    ensure!(
        matches!(status, "rough" | "developed" | "selected" | "killed"),
        "概念状态无效"
    );
    let cid = input["id"]
        .as_str()
        .or_else(|| existing.and_then(|e| e["id"].as_str()))
        .map(str::to_owned)
        .unwrap_or_else(id);
    ensure!(
        !cid.is_empty()
            && cid.len() <= 100
            && cid
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b)),
        "概念 ID 格式无效"
    );
    let created = existing
        .map(|e| num(&e["createdAt"], 0.) as u64)
        .filter(|v| *v > 0)
        .unwrap_or_else(|| now().unwrap_or(0));
    Ok(json!({
        "id": cid,
        "title": title,
        "oneLine": one_line,
        "truth": truth,
        "brandIdea": brand_idea,
        "participant": participant,
        "want": want,
        "ritual": ritual,
        "evidence": evidence,
        "reframe": reframe,
        "risk": risk,
        "cost": cost,
        "status": status,
        "createdAt": created,
        "updatedAt": now().unwrap_or(created)
    }))
}
pub(super) fn apply(input: &Value) -> Result<Value> {
    let before = &input["document"];
    let operations = input["args"]["operations"]
        .as_array()
        .context("每次操作应为 1–30 项，总内容不超过 100 KB")?;
    ensure!(
        !operations.is_empty()
            && operations.len() <= 30
            && input["args"]["operations"]
                .to_string()
                .encode_utf16()
                .count()
                <= 100000,
        "每次操作应为 1–30 项，总内容不超过 100 KB"
    );
    let mut p = before.clone();
    for (i, op) in operations.iter().enumerate() {
        ensure!(op.is_object(), "操作格式无效");
        if let Some(kind) = op.get("expectedKind") {
            ensure!(
                node(&p, &op["id"]).is_some_and(|n| &n["kind"] == kind),
                "Node kind does not match this tool"
            );
        }
        match text(&op["op"]) {
            "move_clip" | "retime_clip" | "slip_clip" => timeline::agent_edit(&mut p, op)?,
            "add_node" => {
                let nid = checked_text(&op["id"], 80)?;
                ensure!(
                    !nid.is_empty()
                        && nid
                            .bytes()
                            .all(|b| b.is_ascii_alphanumeric() || b"_-".contains(&b))
                        && node(&p, &op["id"]).is_none(),
                    "节点 ID 已存在或格式无效"
                );
                let kind = checked_text(&op["kind"], 20)?;
                ensure!(
                    ["text", "shot", "note", "asset", "screenplay"].contains(&kind),
                    "节点类型无效"
                );
                ensure!(
                    kind != "screenplay"
                        || !rows(&p["nodes"]).iter().any(|n| n["kind"] == "screenplay"),
                    "项目已有脚本，请使用 mstudio_update_screenplay 修改现有脚本并保留 ID"
                );
                if kind == "asset" {
                    ensure!(
                        rows(&p["assets"]).iter().any(|a| a["id"] == op["assetId"]),
                        "只能关联本项目已有素材"
                    );
                }
                let count = rows(&p["nodes"]).len();
                let title = checked_text(&op["title"], 300)?;
                let body = op
                    .get("text")
                    .map(|v| checked_text(v, 24000))
                    .transpose()?
                    .unwrap_or("");
                let mut n = json!({"id":nid,"kind":kind,"title":title,"text":body,"x":op.get("x").map(checked_number).transpose()?.unwrap_or(400.+(count%3)as f64*360.),"y":op.get("y").map(checked_number).transpose()?.unwrap_or((count/3)as f64*300.),"width":if kind=="text"{360}else{280},"height":if kind=="text"{300}else{218}});
                if let Some(a) = op.get("assetId") {
                    n["assetId"] = a.clone();
                }
                let n = creative::extras(&p, n, op)?;
                array(&mut p["nodes"]).push(n);
            }
            "update_node" => {
                let ni = index(&p, "nodes", &op["id"])?;
                let mut n = p["nodes"][ni].clone();
                for (k, max) in [("title", 300), ("text", 24000)] {
                    if let Some(v) = op.get(k) {
                        checked_text(v, max)?;
                        n[k] = v.clone();
                    }
                }
                let n = creative::extras(&p, n, op)?;
                p["nodes"][ni] = n;
                if op.get("resultAssetId").is_some() && p["nodes"][ni]["shot"].is_object() {
                    timeline::choose(&mut p, &op["id"], &op["resultAssetId"])?;
                }
            }
            "remove_node" => remove_nodes(&mut p, &[op["id"].clone()]),
            "remove_concepts" => {
                let ids = op["ids"].as_array().context("ids 必须是数组")?;
                ensure!(!ids.is_empty() && ids.len() <= 30, "每次删除 1–30 个概念");
                for cid in ids {
                    ensure!(cid.is_string(), "概念 ID 格式无效");
                    ensure!(
                        rows(&p["concepts"]).iter().any(|c| &c["id"] == cid),
                        "概念不存在，请重新读取创意板"
                    );
                }
                array(&mut p["concepts"]).retain(|c| !ids.contains(&c["id"]));
            }
            "save_concepts" => {
                let mode = text(&op["mode"]);
                ensure!(
                    mode.is_empty() || mode == "merge" || mode == "replace",
                    "概念保存方式无效"
                );
                let incoming = op["concepts"].as_array().context("concepts 必须是数组")?;
                ensure!(
                    !incoming.is_empty() && incoming.len() <= 20,
                    "每次保存 1–20 个概念"
                );
                let mut cards = if mode == "replace" {
                    Vec::new()
                } else {
                    rows(&p["concepts"]).to_vec()
                };
                for item in incoming {
                    let existing = item["id"]
                        .as_str()
                        .and_then(|id| cards.iter().find(|c| c["id"] == id))
                        .cloned();
                    let card = concept_card(item, existing.as_ref())?;
                    if let Some(index) = cards.iter().position(|c| c["id"] == card["id"]) {
                        cards[index] = card;
                    } else {
                        cards.push(card);
                    }
                }
                ensure!(cards.len() <= 30, "概念板上限为 30 个方向");
                p["concepts"] = json!(cards);
            }
            "set_brief" => p["brief"] = json!(checked_text(&op["text"], 24000)?),
            "choose_take" => timeline::choose(&mut p, &op["id"], &op["assetId"])?,
            "assemble_screenplay" => timeline::assemble(&mut p, &op["id"])?,
            "request_generation" => generation::request(&mut p, op, &input["context"], i)?,
            "archive_generation" | "update_generation" | "regenerate_generation" => {
                generation::edit(&mut p, op, &input["context"], i)?
            }
            "append_clip" => {
                let a = asset(&p, &op["assetId"]).context("素材不存在")?.clone();
                ensure!(
                    a["inLibrary"] != false,
                    "素材已从素材库移除，不能新增引用；请先恢复素材"
                );
                timeline::append(&mut p, &a, None, None, None)?;
                let ci = rows(&p["clips"])
                    .len()
                    .checked_sub(1)
                    .context("素材不能添加到时间线")?;
                p["clips"][ci] = timeline::patch(&p, p["clips"][ci].clone(), op)?;
            }
            "update_clip" => {
                let ci = index(&p, "clips", &op["id"])?;
                p["clips"][ci] = timeline::patch(&p, p["clips"][ci].clone(), op)?;
            }
            "remove_clip" => array(&mut p["clips"]).retain(|c| c["id"] != op["id"]),
            "set_transition" => timeline::transition(&mut p, op)?,
            "set_creation" => {
                let mut c = if p["creation"].is_object() {
                    p["creation"].clone()
                } else {
                    json!({"intent":p["brief"],"essential":"","preserve":"","stage":"planning"})
                };
                for k in ["intent", "essential", "preserve"] {
                    if let Some(v) = op.get(k) {
                        checked_text(v, 6000)?;
                        c[k] = v.clone();
                    }
                }
                if let Some(stage) = op.get("stage") {
                    ensure!(
                        matches!(text(stage), "planning" | "production" | "editing"),
                        "创作阶段无效"
                    );
                    c["stage"] = stage.clone();
                }
                p["creation"] = c;
            }
            "set_references" => {
                let ni = index(&p, "nodes", &op["id"])?;
                let old = p["nodes"][ni]["references"].clone();
                let mode = op["referenceMode"].as_str().unwrap_or("replace");
                ensure!(
                    matches!(mode, "replace" | "upsert" | "remove"),
                    "参考编辑方式无效"
                );
                let refs = if mode == "remove" {
                    let ids = op["assetIds"]
                        .as_array()
                        .context("请指定要解除引用的素材 ID")?;
                    ensure!(
                        ids.iter().all(Value::is_string),
                        "请指定要解除引用的素材 ID"
                    );
                    json!(
                        rows(&old)
                            .iter()
                            .filter(|r| !ids.contains(&r["assetId"]))
                            .cloned()
                            .collect::<Vec<_>>()
                    )
                } else {
                    let mut incoming = op["references"].clone();
                    if mode == "upsert" {
                        for r in array(&mut incoming) {
                            if let Some(prev) =
                                rows(&old).iter().find(|v| v["assetId"] == r["assetId"])
                            {
                                let mut merged = prev.clone();
                                assign(&mut merged, r);
                                *r = merged;
                            }
                        }
                    }
                    let incoming = creative::references(
                        &p,
                        &incoming,
                        &old,
                        p["nodes"][ni]["kind"] == "shot",
                    )?;
                    if mode == "replace" {
                        incoming
                    } else {
                        let mut merged = rows(&old).to_vec();
                        for r in rows(&incoming) {
                            if let Some(i) =
                                merged.iter().position(|old| old["assetId"] == r["assetId"])
                            {
                                assign(&mut merged[i], r);
                            } else {
                                merged.push(r.clone());
                            }
                        }
                        json!(merged)
                    }
                };
                p["nodes"][ni]["references"] = refs;
                if p["nodes"][ni]["shot"].is_object() {
                    p["nodes"][ni]["shot"]["visualChanged"] = json!(true);
                }
            }
            "add_track" => {
                let tid = op.get("id").cloned().unwrap_or(json!(id()));
                checked_text(&tid, 80)?;
                ensure!(
                    !rows(&p["tracks"]).iter().any(|t| t["id"] == tid)
                        && matches!(text(&op["trackKind"]), "video" | "audio"),
                    "轨道 ID 或类型无效"
                );
                let name = checked_text(&op["title"], 100)?;
                array(&mut p["tracks"]).push(json!({"id":tid,"kind":op["trackKind"],"name":name}));
            }
            "update_track" => {
                let ti = index(&p, "tracks", &op["id"])?;
                if let Some(v) = op.get("title") {
                    p["tracks"][ti]["name"] = json!(checked_text(v, 100)?);
                }
                for k in ["muted", "hidden"] {
                    if let Some(v) = op.get(k) {
                        ensure!(v.is_boolean(), "轨道开关无效");
                        p["tracks"][ti][k] = v.clone();
                    }
                }
            }
            "add_caption" | "update_caption" => {
                let old = rows(&p["captions"])
                    .iter()
                    .position(|c| c["id"] == op["id"]);
                ensure!(
                    if op["op"] == "add_caption" {
                        old.is_none()
                    } else {
                        old.is_some()
                    },
                    "字幕不存在或 ID 重复"
                );
                let previous = old.map(|i| p["captions"][i].clone()).unwrap_or(json!({}));
                let cid = previous
                    .get("id")
                    .or_else(|| op.get("id"))
                    .cloned()
                    .unwrap_or(json!(id()));
                let body = op.get("text").unwrap_or(&previous["text"]);
                checked_text(body, 1000)?;
                let start = checked_number(op.get("start").unwrap_or(&previous["start"]))?;
                let end = checked_number(op.get("end").unwrap_or(&previous["end"]))?;
                ensure!(
                    !text(body).trim().is_empty() && start >= 0. && end > start,
                    "字幕文字或时间无效"
                );
                let c = json!({"id":cid,"text":body,"start":start,"end":end});
                if let Some(i) = old {
                    p["captions"][i] = c;
                } else {
                    array(&mut p["captions"]).push(c);
                }
            }
            "remove_caption" => array(&mut p["captions"]).retain(|c| c["id"] != op["id"]),
            _ => anyhow::bail!("不支持的工程操作"),
        }
    }
    ensure!(rows(&p["nodes"]).len() <= 5000, "节点数量超过上限");
    creative::validate_order(&p)?;
    for (key, t) in p["production"]["drafts"]
        .as_object()
        .into_iter()
        .flat_map(|m| m.iter())
    {
        let old = before["production"]["drafts"].get(key);
        if old.is_some_and(|old| equal(old, t)) {
            continue;
        }
        if let Some(old) = old {
            let mut a = t.clone();
            obj(&mut a).remove("hiddenFromList");
            if let Some(v) = old.get("hiddenFromList") {
                a["hiddenFromList"] = v.clone();
            }
            if equal(&a, old) {
                continue;
            }
        }
        if text(&t["modelId"]).is_empty() {
            continue;
        }
        let m = rows(&input["models"])
            .iter()
            .find(|m| m["id"] == t["modelId"] && m["enabled"] == true)
            .context("Selected generation model missing, disabled or wrong kind")?;
        ensure!(
            m["kind"] == t["kind"],
            "Selected generation model missing, disabled or wrong kind"
        );
        let model = serde_json::from_value(m.clone())?;
        super::super::generation_parameters::resolve(&model, t, &p)?;
    }
    p["revision"] =
        json!(num(&before["revision"], 0.) + u8::from(content_changed(before, &p)) as f64);
    let receipt = receipt::receipt(before, &p, operations, &input["executionId"]);
    Ok(json!({"document":p,"receipt":receipt}))
}
