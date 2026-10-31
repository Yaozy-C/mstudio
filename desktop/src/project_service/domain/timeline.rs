use super::*;
pub(super) fn duration(c: &Value) -> f64 {
    (num(&c["trimOut"], 0.) - num(&c["trimIn"], 0.)) / num(&c["speed"], 1.)
}
fn validate(c: &Value, a: &Value) -> bool {
    let n = |k: &str, d: f64| num(&c[k], d);
    ["trimIn", "trimOut", "speed", "volume"]
        .iter()
        .all(|k| c[*k].is_number())
        && n("trimIn", 0.) >= 0.
        && n("trimOut", 0.) > n("trimIn", 0.)
        && (a["kind"] == "image" || n("trimOut", 0.) <= num(&a["duration"], 0.) + 0.001)
        && (0.25..=4.).contains(&n("speed", 1.))
        && (0. ..=1.).contains(&n("volume", 1.))
        && n("start", 0.) >= 0.
        && (0.05..=4.).contains(&n("scale", 1.))
        && (0. ..=1.).contains(&n("opacity", 1.))
        && (-1. ..=2.).contains(&n("x", 0.5))
        && (-1. ..=2.).contains(&n("y", 0.5))
        && n("fadeIn", 0.) >= 0.
        && n("fadeOut", 0.) >= 0.
        && duration(c) <= 3600.
}
fn validate_track(p: &Value, c: &Value) -> Result<()> {
    let a = asset(p, &c["assetId"]).context("素材不存在")?;
    let t = rows(&p["tracks"])
        .iter()
        .find(|t| t["id"] == c["trackId"])
        .context("轨道不存在")?;
    ensure!(
        validate(c, a)
            && (if t["kind"] == "video" {
                a["kind"] != "audio"
            } else {
                a["hasAudio"] == true
            }),
        "片段参数、源素材余量或目标轨道无效"
    );
    Ok(())
}
pub(super) fn append(
    p: &mut Value,
    a: &Value,
    at: Option<f64>,
    track: Option<&Value>,
    shot: Option<&Value>,
) -> Result<()> {
    if matches!(text(&a["kind"]), "text" | "document") {
        return Ok(());
    }
    let kind = if a["kind"] == "audio" {
        "audio"
    } else {
        "video"
    };
    let t = rows(&p["tracks"])
        .iter()
        .find(|t| track == Some(&t["id"]) && t["kind"] == kind)
        .or_else(|| rows(&p["tracks"]).iter().find(|t| t["kind"] == kind))
        .cloned();
    let tid = if let Some(t) = t {
        t["id"].clone()
    } else {
        let tid = id();
        let count = rows(&p["tracks"])
            .iter()
            .filter(|t| t["kind"] == kind)
            .count()
            + 1;
        array(&mut p["tracks"]).push(json!({"id":tid,"kind":kind,"name":format!("{} {count}",if kind=="video"{"画面"}else{"音轨"})}));
        json!(tid)
    };
    let start = at.unwrap_or_else(|| {
        rows(&p["clips"])
            .iter()
            .filter(|c| c["trackId"] == tid)
            .map(|c| num(&c["start"], 0.) + duration(c))
            .fold(0., f64::max)
    });
    let mut c = json!({"id":id(),"assetId":a["id"],"trimIn":0,"trimOut":if a["kind"]=="image"{json!(3)}else{a["duration"].clone()},"speed":1,"volume":1,"start":start,"trackId":tid});
    if let Some(s) = shot {
        c["shotId"] = s.clone();
    }
    array(&mut p["clips"]).push(c);
    Ok(())
}
pub(super) fn patch(p: &Value, mut c: Value, op: &Value) -> Result<Value> {
    for k in [
        "trimIn", "trimOut", "speed", "volume", "start", "x", "y", "scale", "opacity", "fadeIn",
        "fadeOut",
    ] {
        if let Some(v) = op.get(k) {
            checked_number(v)?;
            c[k] = v.clone();
        }
    }
    if let Some(v) = op.get("trackId") {
        checked_text(v, 100)?;
        c["trackId"] = v.clone();
    }
    if let Some(v) = op.get("visual") {
        if v.is_null() {
            obj(&mut c).remove("visual");
        } else {
            let fields = v.as_object().context("调色参数必须是对象")?;
            let mut visual =
                json!({"brightness":0,"contrast":1,"saturation":1,"temperature":0,"effect":"none"});
            if c["visual"].is_object() {
                assign(&mut visual, &c["visual"]);
            }
            for (k, v) in fields {
                match k.as_str() {
                    "grade" => {
                        if v.is_null() {
                            obj(&mut visual).remove("grade");
                        } else {
                            let mut grade = visual.get("grade").cloned().unwrap_or(
                                serde_json::to_value(mstudio::grading::Grade::default())?,
                            );
                            ensure!(v.is_object(), "调色方案必须是对象");
                            assign(&mut grade, v);
                            let typed: mstudio::grading::Grade =
                                serde_json::from_value(grade.clone())?;
                            typed.validate()?;
                            visual["grade"] = grade;
                        }
                    }
                    "effect" => {
                        ensure!(
                            matches!(
                                text(v),
                                "none" | "grayscale" | "sepia" | "blur" | "vignette"
                            ),
                            "不支持的画面特效"
                        );
                        visual[k] = v.clone();
                    }
                    _ => {
                        let (lo, hi) = match k.as_str() {
                            "brightness" => (-0.5, 0.5),
                            "contrast" => (0.5, 1.5),
                            "saturation" => (0., 2.),
                            "temperature" => (-1., 1.),
                            _ => anyhow::bail!("不支持的调色参数：{k}"),
                        };
                        ensure!(
                            v.as_f64().is_some_and(|n| (lo..=hi).contains(&n)),
                            "调色参数 {k} 超出范围"
                        );
                        visual[k] = v.clone();
                    }
                }
            }
            ensure!(
                rows(&p["tracks"])
                    .iter()
                    .any(|t| t["id"] == c["trackId"] && t["kind"] == "video"),
                "片段参数或轨道无效"
            );
            c["visual"] = visual;
        }
    }
    validate_track(p, &c)?;
    Ok(c)
}
pub(super) fn agent_edit(p: &mut Value, op: &Value) -> Result<()> {
    let index = rows(&p["clips"])
        .iter()
        .position(|c| c["id"] == op["id"])
        .context("片段不存在")?;
    let old = p["clips"][index].clone();
    let mut edited = old.clone();
    for k in ["ripple", "allowOverlap"] {
        ensure!(op.get(k).is_none_or(Value::is_boolean), "{k}必须为布尔值");
    }
    let end = num(&old["start"], 0.) + duration(&old);
    let mut shift = 0.;
    match text(&op["op"]) {
        "move_clip" => {
            let start = checked_number(&op["start"])?;
            ensure!(start >= 0., "时间线位置不能为负数");
            let fps = num(&p["fps"], 30.);
            edited["start"] = json!((start * fps).round() / fps);
            if let Some(v) = op.get("trackId") {
                checked_text(v, 100)?;
                edited["trackId"] = v.clone();
            }
        }
        "retime_clip" => {
            edited["speed"] = json!(checked_number(&op["speed"])?);
            if op["ripple"] == true {
                shift = duration(&edited) - duration(&old);
            }
        }
        "slip_clip" => {
            let offset = checked_number(&op["sourceOffset"])?;
            edited["trimIn"] = json!(num(&old["trimIn"], 0.) + offset);
            edited["trimOut"] = json!(num(&old["trimOut"], 0.) + offset);
        }
        _ => anyhow::bail!("不支持的剪辑操作"),
    };
    validate_track(p, &edited)?;
    if shift != 0. {
        ensure!(
            !rows(&p["clips"]).iter().any(|c| c["id"] != old["id"]
                && c["trackId"] == old["trackId"]
                && num(&c["start"], 0.) < end - 1e-8
                && num(&c["start"], 0.) + duration(c) > end + 1e-8),
            "变速片段尾部有重叠片段，无法安全顺移；请先整理接缝"
        );
    }
    let mut clips = rows(&p["clips"]).to_vec();
    for c in &mut clips {
        if c["id"] == old["id"] {
            *c = edited.clone();
        } else if shift != 0.
            && c["trackId"] == old["trackId"]
            && num(&c["start"], 0.) >= end - 1e-8
        {
            c["start"] = json!(num(&c["start"], 0.) + shift);
        }
    }
    if op["op"] != "slip_clip" && op["allowOverlap"] != true {
        ensure!(
            !clips.iter().any(|c| c["id"] != edited["id"]
                && c["trackId"] == edited["trackId"]
                && num(&c["start"], 0.) < num(&edited["start"], 0.) + duration(&edited) - 1e-8
                && num(&c["start"], 0.) + duration(c) > num(&edited["start"], 0.) + 1e-8),
            "操作会产生片段重叠；变速可指定 ripple，明确需要叠加时指定 allowOverlap"
        );
    }
    let before = clips.clone();
    for c in &mut clips {
        if !c["transition"].is_object() {
            continue;
        }
        let valid = before
            .iter()
            .find(|a| a["id"] == c["transition"]["fromClipId"])
            .is_some_and(|a| {
                a["trackId"] == c["trackId"]
                    && (num(&a["start"], 0.) + duration(a) - num(&c["start"], 0.)).abs()
                        <= 0.5 / num(&p["fps"], 30.)
                    && num(&c["transition"]["duration"], 0.) <= duration(a).min(duration(c))
            });
        if !valid {
            obj(c).remove("transition");
        }
    }
    p["clips"] = json!(clips);
    Ok(())
}
pub(super) fn transition(p: &mut Value, op: &Value) -> Result<()> {
    let i = rows(&p["clips"])
        .iter()
        .position(|c| c["id"] == op["id"])
        .context("转场目标片段不存在")?;
    if op["kind"].is_null() {
        obj(&mut p["clips"][i]).remove("transition");
        return Ok(());
    }
    let right = &p["clips"][i];
    let left = rows(&p["clips"])
        .iter()
        .find(|c| c["id"] == op["fromClipId"])
        .context("请选择底层画面轨上两个相邻片段的接缝")?;
    let base = rows(&p["tracks"]).iter().find(|t| t["kind"] == "video");
    ensure!(
        base.is_some_and(|t| t["id"] == right["trackId"])
            && left["trackId"] == right["trackId"]
            && (num(&left["start"], 0.) + duration(left) - num(&right["start"], 0.)).abs()
                <= 0.5 / num(&p["fps"], 30.)
            && !rows(&p["clips"])
                .iter()
                .any(|c| c["trackId"] == right["trackId"]
                    && num(&c["start"], 0.) > num(&left["start"], 0.)
                    && num(&c["start"], 0.) < num(&right["start"], 0.)),
        "请选择底层画面轨上两个相邻片段的接缝"
    );
    let kind = text(&op["kind"]);
    ensure!(
        [
            "fade",
            "fadeblack",
            "fadewhite",
            "wipeleft",
            "wiperight",
            "slideleft",
            "slideright",
            "smoothleft",
            "smoothright",
            "circleopen",
            "circleclose",
            "dissolve",
            "custom"
        ]
        .contains(&kind),
        "不支持的转场类型"
    );
    let seconds = checked_number(&op["duration"])?;
    ensure!(
        seconds >= 0.05 && seconds <= 3f64.min(duration(left)).min(duration(right)),
        "转场须为 0.05–3 秒，且不超过相邻片段时长"
    );
    for c in [left, right] {
        ensure!(
            num(&c["x"], 0.5) == 0.5
                && num(&c["y"], 0.5) == 0.5
                && num(&c["scale"], 1.) == 1.
                && num(&c["opacity"], 1.) == 1.,
            "转场目前需要全幅、不透明的画面，请恢复位置与缩放"
        );
    }
    ensure!(
        !rows(&p["clips"])
            .iter()
            .any(|c| c["trackId"] == right["trackId"]
                && c["id"] != left["id"]
                && c["id"] != right["id"]
                && num(&c["start"], 0.) < num(&right["start"], 0.) + seconds / 2.
                && num(&c["start"], 0.) + duration(c) > num(&right["start"], 0.) - seconds / 2.),
        "接缝处存在重叠片段"
    );
    let mut transition = json!({"fromClipId":op["fromClipId"],"kind":kind,"duration":seconds});
    if kind == "custom" {
        ensure!(op["design"].is_object(), "自定义转场必须提供 design");
        let design: mstudio::transition_design::Design =
            serde_json::from_value(op["design"].clone())?;
        design.validate()?;
        transition["design"] = serde_json::to_value(design)?;
    } else {
        ensure!(op.get("design").is_none(), "只有自定义转场接受 design");
    }
    p["clips"][i]["transition"] = transition;
    Ok(())
}
fn take(p: &Value, n: &Value, aid: &Value) -> Result<Value> {
    let t = rows(&n["shot"]["takes"])
        .iter()
        .find(|t| &t["assetId"] == aid)
        .context("镜头结果丢失或区间无效")?;
    let a = asset(p, aid)
        .filter(|a| a["kind"] != "audio")
        .context("镜头结果丢失或区间无效")?;
    ensure!(
        validate(
            &json!({"trimIn":t["trimIn"],"trimOut":t["trimOut"],"speed":1,"volume":0,"start":0}),
            a
        ),
        "镜头结果丢失或区间无效"
    );
    Ok(t.clone())
}
pub(super) fn choose(p: &mut Value, nid: &Value, aid: &Value) -> Result<()> {
    let ni = rows(&p["nodes"])
        .iter()
        .position(|n| &n["id"] == nid && n["shot"].is_object())
        .context("镜头不存在")?;
    let n = p["nodes"][ni].clone();
    let t = take(p, &n, aid)?;
    let visual: Vec<_> = rows(&p["tracks"])
        .iter()
        .filter(|t| t["kind"] == "video")
        .map(|t| t["id"].clone())
        .collect();
    let affected: Vec<_> = rows(&p["clips"])
        .iter()
        .filter(|c| &c["shotId"] == nid && visual.contains(&c["trackId"]))
        .cloned()
        .collect();
    let changing: HashSet<_> = affected
        .iter()
        .filter(|c| &c["assetId"] != aid)
        .map(|c| c["id"].to_string())
        .collect();
    if !changing.is_empty() {
        let start = affected
            .iter()
            .map(|c| num(&c["start"], 0.))
            .fold(f64::INFINITY, f64::min);
        let end = affected
            .iter()
            .map(|c| num(&c["start"], 0.) + duration(c))
            .fold(0., f64::max);
        let speed = (num(&t["trimOut"], 0.) - num(&t["trimIn"], 0.)) / (end - start);
        ensure!(
            (0.25..=4.).contains(&speed),
            "候选时长与当前镜头差距过大，请先调整素材区间"
        );
        let audio = id();
        let mut preserved = Vec::new();
        let mut clips = rows(&p["clips"]).to_vec();
        let a = asset(p, aid).context("素材不存在")?;
        for c in &mut clips {
            if !changing.contains(&c["id"].to_string()) {
                continue;
            }
            if asset(p, &c["assetId"]).is_some_and(|a| a["hasAudio"] == true)
                && num(&c["volume"], 0.) > 0.
            {
                let mut original = c.clone();
                original["id"] = json!(id());
                original["trackId"] = json!(audio);
                obj(&mut original).remove("shotId");
                preserved.push(original);
            }
            let d = duration(c);
            let trim = num(&t["trimIn"], 0.) + (num(&c["start"], 0.) - start) * speed;
            c["assetId"] = aid.clone();
            c["trimIn"] = json!(trim);
            c["trimOut"] = json!(trim + d * speed);
            c["speed"] = json!(speed);
            c["volume"] = json!(0);
            ensure!(validate(c, a), "替换后片段超出素材范围");
        }
        if !preserved.is_empty() {
            clips.extend(preserved);
            array(&mut p["tracks"]).push(
                json!({"id":audio,"name":format!("{} · 原声音",text(&n["title"])),"kind":"audio"}),
            );
        }
        p["clips"] = json!(clips);
    }
    p["nodes"][ni]["resultAssetId"] = aid.clone();
    if !text(&t["basis"]).is_empty() {
        p["nodes"][ni]["shot"]["visualChanged"] = json!(t["basis"] != creative::shot_basis(&n));
    }
    Ok(())
}
pub(super) fn assemble(p: &mut Value, sid: &Value) -> Result<()> {
    let mut shots: Vec<_> = rows(&p["nodes"])
        .iter()
        .filter(|n| n["kind"] == "shot" && &n["shot"]["screenplayId"] == sid)
        .cloned()
        .collect();
    shots.sort_by(|a, b| num(&a["shot"]["order"], 0.).total_cmp(&num(&b["shot"]["order"], 0.)));
    ensure!(!shots.is_empty(), "脚本还没有镜头");
    ensure!(
        shots.iter().all(|n| n["resultAssetId"].is_string()),
        "先为每个镜头选用画面，再编排到时间线"
    );
    let visual: Vec<_> = rows(&p["tracks"])
        .iter()
        .filter(|t| t["kind"] == "video")
        .map(|t| t["id"].clone())
        .collect();
    let missing: HashSet<_> = shots
        .iter()
        .filter(|n| {
            !rows(&p["clips"])
                .iter()
                .any(|c| c["shotId"] == n["id"] && visual.contains(&c["trackId"]))
        })
        .map(|n| n["id"].to_string())
        .collect();
    if missing.is_empty() {
        return Ok(());
    }
    let track = json!(id());
    array(&mut p["tracks"]).push(json!({"id":track,"kind":"video","name":"分镜画面"}));
    let mut start: f64 = 0.;
    for n in shots {
        if !missing.contains(&n["id"].to_string()) {
            start = rows(&p["clips"])
                .iter()
                .filter(|c| c["shotId"] == n["id"] && visual.contains(&c["trackId"]))
                .map(|c| num(&c["start"], 0.) + duration(c))
                .fold(start, f64::max);
            continue;
        }
        let a = asset(p, &n["resultAssetId"]).context("素材不存在")?.clone();
        let t = take(p, &n, &n["resultAssetId"])?;
        let speed =
            (num(&t["trimOut"], 0.) - num(&t["trimIn"], 0.)) / num(&n["shot"]["duration"], 1.);
        ensure!(
            (0.25..=4.).contains(&speed),
            "「{}」素材与计划时长不匹配，请先调整镜头时长",
            text(&n["title"])
        );
        append(p, &a, Some(start), Some(&track), Some(&n["id"]))?;
        let c = array(&mut p["clips"]).last_mut().context("片段不存在")?;
        c["trimIn"] = t["trimIn"].clone();
        c["trimOut"] = t["trimOut"].clone();
        c["speed"] = json!(speed);
        start += num(&n["shot"]["duration"], 0.);
    }
    Ok(())
}
