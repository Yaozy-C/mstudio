use super::*;
pub(super) fn validate_order(p: &Value) -> Result<()> {
    let mut seen = HashSet::new();
    for n in rows(&p["nodes"]).iter().filter(|n| n["shot"].is_object()) {
        ensure!(
            seen.insert(format!(
                "{}:{}",
                text(&n["shot"]["screenplayId"]),
                n["shot"]["order"]
            )),
            "脚本中已有相同顺序的镜头"
        );
    }
    Ok(())
}
pub(super) fn shot_basis(n: &Value) -> String {
    let refs = if n["references"].is_array() {
        json!(
            rows(&n["references"])
                .iter()
                .map(|r| json!([r["assetId"], r["purpose"], r["start"], r["end"]]))
                .collect::<Vec<_>>()
        )
    } else {
        Value::Null
    };
    let mut v = vec![
        n["text"].clone(),
        n["shot"]["dialogue"].clone(),
        n["shot"]["duration"].clone(),
        refs,
    ];
    if !text(&n["shot"]["prompt"]).is_empty() {
        v.push(n["shot"]["prompt"].clone());
    }
    stringify(&json!(v))
}
pub(super) fn paragraph_basis(s: &Value) -> String {
    let mut a = vec![
        s["title"].clone(),
        s["action"].clone(),
        s["dialogue"].clone(),
        s["sound"].clone(),
    ];
    if let Some(v) = s.get("duration") {
        a.push(v.clone());
    }
    if !text(&s["onScreenText"]).is_empty() {
        a.push(json!({"onScreenText":s["onScreenText"]}));
    }
    stringify(&json!(a))
}
pub(super) fn source<'a>(p: &'a Value, n: &Value) -> Option<&'a Value> {
    let screenplay = node(p, &n["shot"]["screenplayId"])?;
    rows(&screenplay["screenplay"]["script"])
        .iter()
        .find(|s| s["id"] == n["shot"]["scriptId"])
}
pub(super) fn prompt_basis(p: &Value, n: &Value) -> String {
    let screenplay = node(p, &n["shot"]["screenplayId"]);
    let frames = if n["shot"]["frames"].is_array() {
        json!(
            rows(&n["shot"]["frames"])
                .iter()
                .map(|f| json!([f["assetId"], f["prompt"]]))
                .collect::<Vec<_>>()
        )
    } else {
        Value::Null
    };
    let mut a = vec![
        n["text"].clone(),
        n["shot"]["dialogue"].clone(),
        n["shot"]["duration"].clone(),
        frames,
        n["references"].clone(),
        screenplay
            .map(|s| s["title"].clone())
            .unwrap_or(Value::Null),
        p["creation"]["intent"].clone(),
        p["creation"]["essential"].clone(),
        p["creation"]["preserve"].clone(),
        p["width"].clone(),
        p["height"].clone(),
    ];
    if let Some(s) = source(p, n) {
        a.push(s.clone());
    }
    stringify(&json!(a))
}
pub(super) fn script_changed(p: &Value, n: &Value) -> bool {
    source(p, n)
        .map(|s| n["shot"]["scriptBasis"] != paragraph_basis(s))
        .unwrap_or(!text(&n["shot"]["scriptId"]).is_empty())
}
fn unique_ids(v: &Value, label: &str) -> Result<Vec<Value>> {
    let a = v
        .as_array()
        .with_context(|| format!("{label}必须是不重复的段落 ID 数组"))?;
    ensure!(
        a.iter().all(Value::is_string)
            && a.iter().map(Value::to_string).collect::<HashSet<_>>().len() == a.len(),
        "{label}必须是不重复的段落 ID 数组"
    );
    Ok(a.clone())
}
fn write_script(old: &Value, patch: &Value) -> Result<Value> {
    let mut next = rows(old).to_vec();
    let mut seen = HashSet::new();
    if let Some(raw) = patch.get("script") {
        let rows = raw.as_array().context("脚本段落格式无效")?;
        ensure!(rows.len() <= 200, "脚本段落格式无效");
        for item in rows {
            let sid = checked_text(&item["id"], 100)?;
            ensure!(
                !sid.trim().is_empty() && seen.insert(sid),
                "脚本段落 ID 无效或重复"
            );
            let index = next.iter().position(|s| s["id"] == item["id"]);
            let mut paragraph = index
                .map(|i| next[i].clone())
                .unwrap_or(json!({"id":sid,"title":"","action":"","dialogue":"","sound":""}));
            for k in ["title", "action", "onScreenText", "dialogue", "sound"] {
                if let Some(v) = item.get(k) {
                    checked_text(v, 6000)?;
                    paragraph[k] = v.clone();
                }
            }
            if let Some(v) = item.get("duration") {
                let d = checked_number(v)?;
                ensure!((0.01..=3600.).contains(&d), "脚本段落时长须为 0.01–3600 秒");
                paragraph["duration"] = v.clone();
            }
            if let Some(i) = index {
                next[i] = paragraph;
            } else {
                next.push(paragraph);
            }
        }
    }
    ensure!(next.len() <= 200, "脚本段落不能超过 200 个");
    if let Some(v) = patch.get("removeParagraphIds") {
        let remove = unique_ids(v, "删除列表")?;
        ensure!(
            remove
                .iter()
                .all(|id| rows(old).iter().any(|s| &s["id"] == id)),
            "要删除的段落不存在，请重新读取脚本"
        );
        ensure!(
            !rows(&patch["script"])
                .iter()
                .any(|s| remove.contains(&s["id"])),
            "不能同时修改和删除同一段"
        );
        next.retain(|s| !remove.contains(&s["id"]));
    }
    if let Some(v) = patch.get("paragraphOrder") {
        let order = unique_ids(v, "排序列表")?;
        ensure!(
            order.len() == next.len() && order.iter().all(|id| next.iter().any(|s| &s["id"] == id)),
            "排序必须包含操作后全部段落 ID，且每个只出现一次"
        );
        next = order
            .iter()
            .map(|id| next.iter().find(|s| &s["id"] == id).unwrap().clone())
            .collect();
    }
    Ok(json!(next))
}
pub(super) fn references(p: &Value, raw: &Value, existing: &Value, shot: bool) -> Result<Value> {
    let refs = raw.as_array().context("参考数量无效")?;
    let mut seen = HashSet::new();
    let mut out = Vec::new();
    for r in refs {
        let a = asset(p, &r["assetId"]).context("只能选择项目中的图片或视频参考")?;
        ensure!(
            matches!(text(&a["kind"]), "image" | "video" | "audio"),
            "只能选择项目中的图片或视频参考"
        );
        ensure!(
            !shot
                || crate::project_service::asset_roles::is_shot_asset(a)
                || rows(&p["previsAssets"])
                    .iter()
                    .any(|v| v["id"] == a["id"] && v["missing"] != true),
            "镜头只能关联已确认资产，请先在资产库建立资产。"
        );
        ensure!(
            a["inLibrary"] != false || rows(existing).iter().any(|r| r["assetId"] == a["id"]),
            "素材已从素材库移除，不能新增引用；请先恢复素材"
        );
        checked_text(&r["purpose"], 400)?;
        ensure!(seen.insert(a["id"].to_string()), "参考素材重复");
        let mut reference = select(r, &["assetId", "purpose"]);
        if a["kind"] == "video" {
            let start = r
                .get("start")
                .map(checked_number)
                .transpose()?
                .unwrap_or(0.);
            let end = r
                .get("end")
                .map(checked_number)
                .transpose()?
                .unwrap_or(num(&a["duration"], 0.));
            ensure!(
                start >= 0. && end > start && end <= num(&a["duration"], 0.),
                "视频参考区间无效"
            );
            reference["start"] = json!(start);
            reference["end"] = json!(end);
        }
        out.push(reference);
    }
    Ok(json!(out))
}
pub(super) fn extras(p: &Value, mut n: Value, op: &Value) -> Result<Value> {
    let previous = node(p, &n["id"]);
    for k in ["assetId", "resultAssetId"] {
        if let Some(v) = op.get(k) {
            ensure!(
                previous.is_some_and(|n| &n[k] == v)
                    || !asset(p, v).is_some_and(|a| a["inLibrary"] == false),
                "素材已从素材库移除，不能新增引用；请先恢复素材"
            );
            let a = asset(p, v).context("结果素材不属于项目")?;
            ensure!(rows(&p["assets"]).contains(a), "结果素材不属于项目");
            if k == "assetId" {
                ensure!(n["kind"] == "asset", "素材记录只能关联本项目已有素材");
            }
            n[k] = v.clone();
        }
    }
    if let Some(v) = op.get("references") {
        n["references"] = references(
            p,
            v,
            previous.map(|n| &n["references"]).unwrap_or(&Value::Null),
            n["kind"] == "shot",
        )?;
    }
    if let Some(v) = op.get("screenplay") {
        ensure!(n["kind"] == "screenplay", "只有脚本文档能设置脚本段落");
        let fields = v.as_object().context("脚本或镜头格式无效")?;
        for k in fields.keys() {
            ensure!(
                ["script", "removeParagraphIds", "paragraphOrder"].contains(&k.as_str()),
                "不支持的脚本字段：{k}"
            );
        }
        n["screenplay"] = json!({"script":write_script(&n["screenplay"]["script"],v)?});
    }
    if let Some(v) = op.get("shot") {
        ensure!(n["kind"] == "shot", "只有镜头能设置分镜内容");
        let fields = v.as_object().context("脚本或镜头格式无效")?;
        for k in fields.keys() {
            ensure!(
                [
                    "screenplayId",
                    "scriptId",
                    "order",
                    "duration",
                    "dialogue",
                    "frames",
                    "framePrompt",
                    "prompt"
                ]
                .contains(&k.as_str()),
                "不支持的镜头字段：{k}"
            );
        }
        let old = n["shot"].clone();
        let screenplay = v
            .get("screenplayId")
            .or_else(|| old.get("screenplayId"))
            .context("脚本文字无效或过长")?;
        checked_text(screenplay, 80)?;
        let Some(s) = node(p, screenplay).filter(|n| n["kind"] == "screenplay") else {
            return Err(reject(
                "SCREENPLAY_NOT_FOUND",
                format!(
                    "Shot {} references absent screenplay {}.",
                    text(&n["id"]),
                    text(screenplay)
                ),
                json!({"nodeId":n["id"],"screenplayId":screenplay}),
            ));
        };
        let script = v.get("scriptId").or_else(|| old.get("scriptId"));
        let source = script.and_then(|id| {
            rows(&s["screenplay"]["script"])
                .iter()
                .find(|s| &s["id"] == id)
        });
        if let Some(script) = script.filter(|s| !text(s).is_empty())
            && source.is_none()
        {
            return Err(reject(
                "PARAGRAPH_NOT_FOUND",
                format!(
                    "Shot {} references paragraph {}, which is absent from screenplay {}.",
                    text(&n["id"]),
                    text(script),
                    text(screenplay)
                ),
                json!({"nodeId":n["id"],"screenplayId":screenplay,"paragraphId":script}),
            ));
        }
        let order = v.get("order").unwrap_or(&old["order"]);
        let d = v.get("duration").unwrap_or(&old["duration"]);
        let o = checked_number(order)?;
        let seconds = checked_number(d)?;
        ensure!(
            o.fract() == 0. && (1. ..=999.).contains(&o) && seconds > 0. && seconds <= 3600.,
            "镜头顺序或时长无效"
        );
        let mut shot = if old.is_object() {
            old.clone()
        } else {
            json!({"dialogue":""})
        };
        shot["screenplayId"] = screenplay.clone();
        if let Some(script) = script {
            shot["scriptId"] = script.clone();
        }
        if let Some(s) = source
            && (!old.is_object() || old["scriptId"] != s["id"])
        {
            shot["scriptBasis"] = json!(paragraph_basis(s));
        }
        shot["order"] = order.clone();
        shot["duration"] = d.clone();
        for (k, max) in [
            ("framePrompt", 12000),
            ("dialogue", 6000),
            ("prompt", 12000),
        ] {
            if let Some(v) = v.get(k) {
                checked_text(v, max)?;
                shot[k] = v.clone();
            }
        }
        if let Some(frames) = v.get("frames") {
            let frames = frames.as_array().context("每个镜头最多 50 张分镜图")?;
            ensure!(frames.len() <= 50, "每个镜头最多 50 张分镜图");
            let mut seen = HashSet::new();
            let mut out = Vec::new();
            for f in frames {
                let fields = f.as_object().context("分镜图格式无效")?;
                ensure!(
                    fields
                        .keys()
                        .all(|k| ["assetId", "title", "prompt"].contains(&k.as_str())),
                    "分镜图仅支持 assetId、title、prompt"
                );
                let a = asset(p, &f["assetId"])
                    .filter(|a| a["kind"] == "image" && rows(&p["assets"]).contains(a))
                    .context("分镜图必须是本项目的图片")?;
                ensure!(
                    a["inLibrary"] != false
                        || rows(&old["frames"]).iter().any(|r| r["assetId"] == a["id"]),
                    "素材已从素材库移除，不能新增引用；请先恢复素材"
                );
                ensure!(
                    seen.insert(f["assetId"].to_string()),
                    "同一镜头不能重复关联同一张分镜图"
                );
                checked_text(&f["title"], 300)?;
                if let Some(v) = f.get("prompt") {
                    checked_text(v, 12000)?;
                }
                out.push(f.clone());
            }
            shot["frames"] = json!(out);
        }
        n["shot"] = shot;
        if v.get("prompt").is_some() {
            n["shot"]["promptBasis"] = json!(prompt_basis(p, &n));
        }
    }
    ensure!(
        n["kind"] != "shot" || n["shot"].is_object(),
        "镜头必须关联一个脚本文档"
    );
    if n["kind"] == "screenplay" {
        ensure!(
            text(&op["text"]).trim().is_empty(),
            "脚本文档不支持非空 text；请删除 text，使用结构化 script 段落"
        );
        if !n["screenplay"].is_object() {
            n["screenplay"] = json!({"script":[]});
        }
    }
    if n["shot"].is_object() && previous.is_some_and(|old| shot_basis(old) != shot_basis(&n)) {
        n["shot"]["visualChanged"] = json!(true);
    }
    if n["shot"].is_object()
        && let Some(aid) = op.get("resultAssetId")
    {
        let a = asset(p, aid).context("结果素材不属于项目")?;
        let mut takes = rows(&n["shot"]["takes"]).to_vec();
        takes.retain(|t| t["assetId"] != *aid);
        takes.push(json!({"assetId":aid,"trimIn":0,"trimOut":if a["kind"]=="image"{n["shot"]["duration"].clone()}else{a["duration"].clone()}}));
        n["shot"]["takes"] = json!(takes);
    }
    Ok(n)
}
