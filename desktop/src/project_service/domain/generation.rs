use super::*;
pub(super) fn tasks(p: &Value) -> Vec<&Value> {
    let mut out: Vec<_> = p["production"]["drafts"]
        .as_object()
        .into_iter()
        .flat_map(|m| m.values())
        .filter(|t| !text(&t["turnId"]).is_empty() && num(&t["createdAt"], 0.) != 0.)
        .collect();
    out.sort_by(|a, b| num(&a["createdAt"], 0.).total_cmp(&num(&b["createdAt"], 0.)));
    out
}
pub(super) fn save(p: &mut Value, t: Value) {
    layout::ensure_production(p, "drafts");
    let key = text(&t["key"]).to_owned();
    p["production"]["drafts"][key] = t;
}
pub(super) fn results(t: &Value) -> Vec<Value> {
    let mut ids = rows(&t["resultAssetIds"]).to_vec();
    if !text(&t["resultAssetId"]).is_empty() && !ids.contains(&t["resultAssetId"]) {
        ids.push(t["resultAssetId"].clone());
    }
    let mut seen = HashSet::new();
    ids.retain(|v| seen.insert(v.to_string()));
    ids
}
pub(super) fn outcome(t: &Value) -> Value {
    let ids = results(t);
    let (state, action, trigger) = if t["hiddenFromList"] == true {
        ("finished", "read_details_on_request", "new_user_request")
    } else if t["trackingPaused"] == true
        && !matches!(text(&t["status"]), "UNKNOWN" | "FAILED" | "CANCELLED")
    {
        ("waiting_user", "resume_tracking", "user_action")
    } else {
        match t["status"].as_str().unwrap_or("DRAFT") {
            "DRAFT" | "AWAITING_CONFIRMATION" => {
                ("waiting_user", "confirm_generation", "user_confirmation")
            }
            "UNKNOWN" | "FAILED" | "CANCELLED" => {
                ("finished", "report_outcome", "new_user_request")
            }
            "COMPLETED" if !ids.is_empty() => ("ready", "inspect_result", "result_available"),
            "READY" | "UPLOADING" | "SUBMITTING" | "IN_QUEUE" | "IN_PROGRESS" | "RECEIVING"
            | "CANCEL_REQUESTED" | "COMPLETED" => {
                ("waiting_service", "await_generation", "generation_event")
            }
            _ => ("waiting_user", "resolve_task_state", "user_action"),
        }
    };
    let mut out = select(t, &["kind", "modelId", "parameters", "error"]);
    assign(
        &mut out,
        &json!({"id":t["key"],"title":title(t),"status":if t["status"] == "UNKNOWN" {"FAILED"} else {t["status"].as_str().unwrap_or("DRAFT")},"archived":t["hiddenFromList"]==true,"resultAssetIds":ids,"continuation":{"state":state,"action":action,"trigger":trigger}}),
    );
    if let Some(v) = t.get("targetNodeId").or_else(|| t.get("ownerId")) {
        out["targetId"] = v.clone();
    }
    out
}
pub(super) fn outcomes(p: &Value, keys: &[Value]) -> Vec<Value> {
    keys.iter().map(|k|p["production"]["drafts"].get(text(k)).map(outcome).unwrap_or(json!({"id":k,"status":"REMOVED","resultAssetIds":[],"continuation":{"state":"finished","action":"report_missing_task","trigger":"new_user_request"}}))).collect()
}
pub(super) fn batch(tasks: &[&Value]) -> Value {
    let mut counts = json!({});
    let mut attention = 0;
    for t in tasks {
        let status = t["status"].as_str().unwrap_or("AWAITING_CONFIRMATION");
        counts[status] = json!(counts[status].as_u64().unwrap_or(0) + 1);
        if t["trackingPaused"] == true || matches!(status, "UNKNOWN" | "FAILED") {
            attention += 1;
        }
    }
    json!({"total":tasks.len(),"counts":counts,"attention":attention})
}
fn issue(job: &Value) -> Option<&Value> {
    job["reconciliation"]
        .get("error")
        .or_else(|| job["sync"].get("error"))
        .or_else(|| job.get("error"))
}
pub(super) fn tracking(mut p: Value, job: &Value) -> Value {
    let found = p["production"]["drafts"]
        .as_object()
        .into_iter()
        .flat_map(|m| m.values())
        .find(|t| t["jobId"] == job["id"] || t["submissionId"] == job["id"])
        .cloned();
    if let Some(mut t) = found {
        let error = issue(job);
        if t.get("error") == error
            && (t["trackingPaused"] == true) == (job["sync"]["paused"] == true)
        {
            return p;
        }
        obj(&mut t).remove("error");
        if let Some(v) = error {
            t["error"] = v.clone();
        }
        t["trackingPaused"] = json!(job["sync"]["paused"] == true);
        save(&mut p, t);
    }
    p
}
pub(super) fn result_name(p: &Value, job: &Value) -> Result<String> {
    let snap = &job["shot"]["canvasGeneration"]["task"];
    ensure!(snap.is_object(), "生成任务缺少制作上下文，未写入项目");
    Ok(title(
        p["production"]["drafts"]
            .get(text(&snap["key"]))
            .unwrap_or(snap),
    )
    .to_owned())
}
fn receive(mut p: Value, asset: &Value, source: &Value) -> Result<Value> {
    let snap = &source["canvasGeneration"];
    ensure!(snap.is_object(), "生成任务缺少制作上下文，未写入项目");
    if rows(&p["removedAssetIds"]).contains(&asset["id"]) {
        return Ok(p);
    }
    let current = p["production"]["drafts"]
        .get(text(&snap["task"]["key"]))
        .unwrap_or(&snap["task"]);
    let mut a = asset.clone();
    a["name"] = json!(title(current));
    a["generated"] = json!(true);
    a["usage"] = json!(if current["generationPurpose"] == "asset" {
        "asset"
    } else {
        "material"
    });
    a["inMaterials"] = json!(false);
    if current["generationPurpose"] == "asset" {
        a["assetStatus"] = json!("candidate");
    } else {
        obj(&mut a).remove("assetStatus");
    }
    obj(&mut a).remove("inLibrary");
    let mut task = snap["task"].clone();
    obj(&mut task).remove("title");
    if let Some(v) = current.get("title") {
        task["title"] = v.clone();
    }
    if task["generationPurpose"] == "asset" {
        obj(&mut task).remove("ownerId");
    }
    ensure!(a["kind"] == task["kind"], "返回的媒体类型与生成任务不一致");
    let owner = task["ownerId"].clone();
    if let Some(mut run) = p["production"]["drafts"].get(text(&task["key"])).cloned()
        && !text(&run["turnId"]).is_empty()
        && run.get("jobId").or_else(|| run.get("submissionId"))
            == task.get("jobId").or_else(|| task.get("submissionId"))
    {
        obj(&mut run).remove("ownerId");
        if !owner.is_null() {
            run["ownerId"] = owner.clone();
        }
        if run["resultAssetId"].is_null() {
            run["resultAssetId"] = a["id"].clone();
        }
        let mut ids = results(&run);
        if !ids.contains(&a["id"]) {
            ids.push(a["id"].clone());
        }
        run["resultAssetIds"] = json!(ids);
        save(&mut p, run);
    }
    let shot = node(&p, &owner).filter(|n| n["shot"].is_object()).cloned();
    if let Some(n) = &shot {
        if rows(&n["shot"]["frames"])
            .iter()
            .chain(rows(&n["shot"]["takes"]))
            .any(|f| f["assetId"] == a["id"])
        {
            return Ok(p);
        }
    } else if rows(&p["assets"]).iter().any(|v| v["id"] == a["id"]) {
        return Ok(p);
    }
    if !rows(&p["assets"]).iter().any(|v| v["id"] == a["id"]) {
        array(&mut p["assets"]).push(a.clone());
    }
    if let Some(n) = shot {
        let index = rows(&p["nodes"])
            .iter()
            .position(|v| v["id"] == n["id"])
            .unwrap();
        if a["kind"] == "image" {
            let count = rows(&n["shot"]["frames"]).len();
            array(&mut p["nodes"][index]["shot"]["frames"])
                .push(json!({"assetId":a["id"],"title":format!("画面 {:02}",count+1)}));
        } else {
            array(&mut p["nodes"][index]["shot"]["takes"]).push(json!({"assetId":a["id"],"trimIn":0,"trimOut":a["duration"],"basis":creative::shot_basis(source),"production":task}));
        }
    } else if task["generationPurpose"] != "asset" {
        let nid = id();
        let preferred = task
            .get("position")
            .cloned()
            .unwrap_or(select(snap, &["x", "y"]));
        let position = layout::free(&p, &task, preferred, Some(layout::size(&a)?))?;
        array(&mut p["nodes"]).push(json!({"id":nid,"kind":"asset","assetId":a["id"],"title":a["name"],"text":"","x":position["x"],"y":position["y"]}));
    }
    Ok(p)
}
fn reconcile(mut p: Value, job: &Value) -> Result<Value> {
    if !job["shot"]["canvasGeneration"].is_object() {
        return Ok(p);
    }
    let Some(current) = p["production"]["drafts"]
        .as_object()
        .into_iter()
        .flat_map(|m| m.values())
        .find(|t| t["jobId"] == job["id"] || t["submissionId"] == job["id"])
        .cloned()
    else {
        return Ok(p);
    };
    let received = results(&current);
    let terminal = matches!(text(&job["status"]), "COMPLETED" | "FAILED" | "CANCELLED");
    let count = if !terminal || (job.get("outputCount").is_none() && job["status"] != "COMPLETED") {
        0
    } else {
        let n = job["outputCount"]
            .as_u64()
            .context("生成任务缺少有效的实际输出数量")?;
        ensure!(
            n >= u64::from(job["status"] == "COMPLETED") && n <= 9_007_199_254_740_991,
            "生成任务缺少有效的实际输出数量"
        );
        n as usize
    };
    ensure!(count <= 10000, "生成输出数量无效");
    let mut missing = false;
    for i in 0..count {
        let a = rows(&job["assets"])
            .get(i)
            .filter(|a| !a.is_null())
            .or_else(|| if i == 0 { job.get("asset") } else { None });
        if let Some(a) = a.filter(|a| a["id"].is_string()) {
            if !received.contains(&a["id"]) {
                p = receive(p, a, &job["shot"])?;
            }
        } else {
            missing = true;
        }
    }
    let mut t = p["production"]["drafts"]
        .get(text(&current["key"]))
        .unwrap_or(&current)
        .clone();
    let status = if missing {
        json!("RECEIVING")
    } else {
        job["status"].clone()
    };
    let error = issue(job);
    let unchanged = t["status"] == status
        && t.get("error") == error
        && (job["requestId"].is_null() || t["requestId"] == job["requestId"])
        && t.get("progress") == job.get("progress");
    if !unchanged {
        t["jobId"] = job["id"].clone();
        t["status"] = status;
        for key in ["error", "progress"] {
            obj(&mut t).remove(key);
        }
        if let Some(e) = error {
            t["error"] = e.clone();
        }
        if let Some(v) = job.get("progress") {
            t["progress"] = v.clone();
        }
        if error.is_none() {
            t["trackingPaused"] = json!(false);
        }
        if let Some(v) = job.get("requestId") {
            t["requestId"] = v.clone();
        }
        save(&mut p, t);
    }
    Ok(tracking(p, job))
}
pub(super) fn recover(document: &Value, jobs: &[Value]) -> Result<Value> {
    let mut p = document.clone();
    let mut failures = Vec::new();
    for original in jobs {
        let mut job = original.clone();
        obj(&mut job).remove("reconciliation");
        if job["sync"]["stage"] == "reconcile" {
            obj(&mut job).remove("sync");
        }
        match reconcile(p.clone(), &job) {
            Ok(mut next) => {
                next["revision"] =
                    json!(num(&p["revision"], 0.) + u8::from(content_changed(&p, &next)) as f64);
                p = next
            }
            Err(error) => {
                let e=json!({"code":"PROJECT_SYNC_FAILED","message":"任务记录尚未写回项目","recovery":"重试同步原任务，无需重新生成。","retryable":true,"stage":"reconcile","details":error.to_string()}).to_string();
                failures.push(json!({"jobId":job["id"],"error":e}));
                let mut failed = original.clone();
                failed["reconciliation"] = json!({"error":e});
                p = tracking(p, &failed);
            }
        }
    }
    Ok(json!({"document":p,"failures":failures}))
}
pub(super) fn references(p: &Value, task: &mut Value, raw: &Value) -> Result<()> {
    let refs = raw.as_array().context("参考素材须为数组")?;
    let mut inputs: Vec<_> = rows(&task["inputs"])
        .iter()
        .filter(|r| r["role"] == "script")
        .cloned()
        .collect();
    let mut seen = HashSet::new();
    for r in refs {
        ensure!(r.is_object() && r["assetId"].is_string(), "参考素材无效");
        let a = asset(p, &r["assetId"]).context("只能使用项目中已有的图片、视频或音频")?;
        ensure!(
            matches!(text(&a["kind"]), "image" | "video" | "audio"),
            "只能使用项目中已有的图片、视频或音频"
        );
        let role = r["role"].as_str().unwrap_or(if a["kind"] == "video" {
            "video-reference"
        } else {
            "reference"
        });
        let allowed = if a["kind"] == "video" {
            vec!["video-reference"]
        } else if a["kind"] == "audio" {
            vec!["reference"]
        } else if task["kind"] == "image" {
            vec!["edit", "reference"]
        } else {
            vec!["reference", "first-frame", "last-frame"]
        };
        ensure!(allowed.contains(&role), "素材用途与类型不匹配");
        ensure!(
            seen.insert((a["id"].clone().to_string(), role.to_owned())),
            "参考素材不能重复"
        );
        let purpose = r.get("purpose").unwrap_or(&a["name"]);
        checked_text(purpose, 400)?;
        let mut input = json!({"key":format!("asset:{}:{role}",text(&a["id"])),"assetId":a["id"],"role":role,"purpose":purpose,"sourceRef":{"kind":"asset","id":a["id"]},"frame":false});
        if let Some(shot) = rows(&p["nodes"]).iter().find(|n| {
            rows(&n["shot"]["frames"])
                .iter()
                .any(|f| f["assetId"] == a["id"])
        }) {
            input["nodeId"] = shot["id"].clone();
            input["frame"] = json!(true);
        }
        if a["kind"] == "video" {
            let start = num(&r["start"], 0.);
            let end = num(&r["end"], num(&a["duration"], 0.));
            ensure!(
                start >= 0. && end > start && end <= num(&a["duration"], 0.),
                "请指定有效的视频参考区间"
            );
            input["start"] = json!(start);
            input["end"] = json!(end);
        } else {
            for k in ["start", "end"] {
                if let Some(v) = r.get(k) {
                    checked_number(v)?;
                    input[k] = v.clone();
                }
            }
        }
        inputs.push(input);
    }
    task["inputs"] = json!(inputs);
    assert_refs(p, task)
}
fn assert_refs(p: &Value, t: &Value) -> Result<()> {
    for r in rows(&t["inputs"]).iter().filter(|r| r["role"] != "script") {
        let a = asset(p, &r["assetId"]).context("参考素材已移除或不可用，请替换该参考")?;
        ensure!(a["missing"] != true, "参考素材已移除或不可用，请替换该参考");
        ensure!(
            text(&a["annotationSourceId"]).is_empty(),
            "内部批注图不能用作生成参考，请使用原图"
        );
        ensure!(
            a["inLibrary"] != false,
            "参考素材已从素材库移除，请移除该参考或先恢复素材"
        );
        ensure!(
            t["generationPurpose"] == "asset"
                || (t["ownerId"].is_null() && t["targetNodeId"].is_null())
                || crate::project_service::asset_roles::is_production_reference(p, a),
            crate::project_service::asset_roles::MATERIAL_REFERENCE_ERROR
        );
    }
    Ok(())
}
pub(super) fn edit(p: &mut Value, op: &Value, context: &Value, index: usize) -> Result<()> {
    let key = text(&op["taskKey"]);
    let mut t = p["production"]["drafts"]
        .get(key)
        .context("任务不存在，请重新查询")?
        .clone();
    if op["op"] == "archive_generation" {
        let archived = op["archived"].as_bool().context("归档状态无效")?;
        ensure!(
            !archived
                || !matches!(
                    text(&t["status"]),
                    "READY"
                        | "UPLOADING"
                        | "SUBMITTING"
                        | "IN_QUEUE"
                        | "IN_PROGRESS"
                        | "RECEIVING"
                        | "CANCEL_REQUESTED"
                ),
            "请先停止生成或等待任务结束，再归档"
        );
        t["hiddenFromList"] = json!(archived);
        save(p, t);
        return Ok(());
    }
    let old = t.clone();
    if op["op"] == "regenerate_generation" {
        ensure!(
            context["turn"]["projectId"] == p["id"] && !context.is_null(),
            "本轮生成上下文已结束，请重新发送要求"
        );
        let action_id = format!("{}:{index}", text(&context["callId"]));
        if t["lastRegenerationId"] == action_id {
            return Ok(());
        }
        ensure!(
            matches!(
                text(&t["status"]),
                "COMPLETED" | "FAILED" | "CANCELLED" | "UNKNOWN"
            ) || (t["jobId"].is_null()
                && t["submissionId"].is_null()
                && matches!(text(&t["status"]), "" | "DRAFT" | "AWAITING_CONFIRMATION")),
            "任务尚未结束，请稍后重试"
        );
        for k in [
            "jobId",
            "submissionId",
            "requestId",
            "error",
            "trackingPaused",
            "progress",
            "resultAssetId",
            "resultAssetIds",
        ] {
            obj(&mut t).remove(k);
        }
        t["hiddenFromList"] = json!(false);
        t["status"] = json!("AWAITING_CONFIRMATION");
        t["lastRegenerationId"] = json!(action_id);
    } else {
        ensure!(
            ["title", "text", "references"]
                .iter()
                .any(|k| op.get(*k).is_some()),
            "请提供任务名称、生成描述或参考素材"
        );
    }
    if op.get("title").is_some() {
        t["title"] = json!(title(op));
    }
    if let Some(v) = op.get("text") {
        ensure!(
            !checked_text(v, 12000)?.trim().is_empty(),
            "请提供完整生成描述（最多 12000 字）"
        );
        t["prompt"] = json!(text(v).trim());
    }
    if let Some(v) = op.get("references") {
        references(p, &mut t, v)?;
    }
    assert_refs(p, &t)?;
    if t["status"] == "READY"
        && t["jobId"].is_null()
        && t["submissionId"].is_null()
        && (t["prompt"] != old["prompt"] || !equal(&t["inputs"], &old["inputs"]))
    {
        t["status"] = json!("AWAITING_CONFIRMATION");
        obj(&mut t).remove("error");
    }
    save(p, t);
    Ok(())
}
pub(super) fn request(p: &mut Value, op: &Value, context: &Value, index: usize) -> Result<()> {
    let turn = &context["turn"];
    ensure!(
        !context.is_null() && turn["projectId"] == p["id"],
        "本轮生成上下文已结束，请重新发送要求"
    );
    let mut key = format!("run:{}:{index}", text(&context["callId"]));
    if p["production"]["drafts"].get(&key).is_some() {
        return Ok(());
    }
    let asset_task = op["generationPurpose"] == "asset";
    ensure!(
        op["generationPurpose"].is_null() || asset_task,
        "生成用途无效"
    );
    ensure!(
        !asset_task
            || (op["mediaKind"] == "image"
                && op.get("id").is_none()
                && op.get("canvasTaskKey").is_none()
                && op["references"].is_array()),
        "资产任务须为独立图片，并明确指定参考素材"
    );
    let kind = op
        .get("mediaKind")
        .or_else(|| turn["task"].get("kind"))
        .and_then(Value::as_str)
        .unwrap_or("image");
    ensure!(matches!(kind, "image" | "video"), "生成类型无效");
    let prompt = checked_text(&op["text"], 12000)?.trim();
    ensure!(!prompt.is_empty(), "请提供完整生成描述（最多 12000 字）");
    let owner = if asset_task {
        Value::Null
    } else {
        op.get("id")
            .or_else(|| turn["task"].get("targetNodeId"))
            .or_else(|| turn["task"].get("ownerId"))
            .cloned()
            .unwrap_or(Value::Null)
    };
    ensure!(
        owner.is_null() || node(p, &owner).is_some_and(|n| n["shot"].is_object()),
        "目标镜头不存在"
    );
    ensure!(
        op["canvasTaskKey"].is_null() || op["canvasTaskKey"] == turn["task"]["key"],
        "请使用本轮消息引用的画布任务"
    );
    let saved = tasks(p)
        .into_iter()
        .find(|t| {
            !asset_task
                && !owner.is_null()
                && t["targetNodeId"] == owner
                && t["kind"] == kind
                && t["status"] == "DRAFT"
                && t["jobId"].is_null()
                && t["submissionId"].is_null()
        })
        .cloned();
    if let Some(s) = &saved {
        key = text(&s["key"]).to_owned();
    }
    let source = saved.as_ref().or_else(|| {
        if !asset_task
            && turn["task"]
                .get("targetNodeId")
                .or_else(|| turn["task"].get("ownerId"))
                .unwrap_or(&Value::Null)
                == &owner
        {
            turn.get("task").filter(|v| v.is_object())
        } else {
            None
        }
    });
    let mut t = source
        .cloned()
        .unwrap_or(json!({"mode":"single","inputs":[]}));
    t["kind"] = json!(kind);
    obj(&mut t).remove("targetNodeId");
    if !owner.is_null() {
        t["targetNodeId"] = owner.clone();
    }
    ensure!(
        kind != "image" || op.get("mode").is_none(),
        "图片任务不接受视频素材组合方式"
    );
    let mode = op
        .get("mode")
        .or_else(|| t.get("mode"))
        .cloned()
        .unwrap_or(json!("single"));
    ensure!(
        matches!(text(&mode), "single" | "ends" | "multi" | "mixed"),
        "素材组合方式无效"
    );
    t["mode"] = if kind == "image" {
        json!("multi")
    } else {
        mode
    };
    if kind == "image" {
        for r in array(&mut t["inputs"]) {
            if matches!(text(&r["role"]), "first-frame" | "last-frame") {
                r["role"] = json!("reference");
            }
        }
    }
    if asset_task {
        t["generationPurpose"] = json!("asset");
    }
    let defaults = if kind == "image" && source.is_none() && !asset_task {
        node(p, &owner).map(|n| {
            rows(&n["references"])
                .iter()
                .filter(|r| {
                    asset(p, &r["assetId"]).is_some_and(|a| {
                        a["kind"] == "image"
                            && crate::project_service::asset_roles::is_shot_asset(a)
                    })
                })
                .cloned()
                .collect::<Vec<_>>()
        })
    } else {
        None
    };
    if let Some(raw) = op
        .get("references")
        .cloned()
        .or_else(|| defaults.map(|v| json!(v)))
    {
        references(p, &mut t, &raw)?;
    }
    assert_refs(p, &t)?;
    let selected = text(&turn["models"][kind]);
    if let Some(v) = op.get("mediaModelId") {
        ensure!(
            v.as_str().is_some_and(|s| !s.trim().is_empty()),
            "媒体模型 ID 无效，请读取模型目录"
        );
        ensure!(
            selected.is_empty() || v == selected,
            "请使用用户选择的模型 {selected}；省略 mediaModelId 可沿用本轮选择。"
        );
    }
    let model = if !selected.is_empty() {
        json!(selected)
    } else {
        op.get("mediaModelId")
            .or_else(|| saved.as_ref().and_then(|s| s.get("modelId")))
            .cloned()
            .unwrap_or(json!(""))
    };
    let inherited = saved
        .as_ref()
        .and_then(|s| s.get("parameters"))
        .or_else(|| {
            if turn["task"]["kind"] == kind && !asset_task {
                turn["task"].get("parameters")
            } else {
                None
            }
        });
    let mut run = json!({"key":key,"title":title(op),"kind":kind,"mode":t["mode"],"inputs":t["inputs"],"prompt":prompt,"modelId":model,"turnId":context["turnId"],"createdAt":now()?+index as u64,"status":"AWAITING_CONFIRMATION"});
    if asset_task {
        run["generationPurpose"] = json!("asset");
    }
    if !owner.is_null() {
        run["targetNodeId"] = owner.clone();
        run["ownerId"] = owner;
    }
    if let Some(v) = turn.get("instruction") {
        run["instruction"] = v.clone();
    }
    if let Some(v) = inherited {
        run["parameters"] = v.clone();
    }
    if let Some(v) = op.get("parameters") {
        if !run["parameters"].is_object() {
            run["parameters"] = json!({});
        }
        assign(&mut run["parameters"], v);
    }
    for old in tasks(p) {
        if saved.as_ref().is_some_and(|s| s["key"] == old["key"]) {
            continue;
        }
        let fields = [
            "targetNodeId",
            "generationPurpose",
            "kind",
            "mode",
            "modelId",
            "prompt",
        ];
        let identity = |t: &Value| {
            rows(&t["inputs"])
                .iter()
                .map(|r| {
                    let mut identity = select(r, &["assetId", "role", "purpose", "start", "end"]);
                    if r["role"] == "script"
                        && let Some(v) = r.get("sourceRef")
                    {
                        identity["sourceRef"] = v.clone();
                    }
                    identity
                })
                .collect::<Vec<_>>()
        };
        if fields.iter().all(|k| old[*k] == run[*k])
            && identity(old) == identity(&run)
            && equal(
                old.get("parameters").unwrap_or(&json!({})),
                run.get("parameters").unwrap_or(&json!({})),
            )
        {
            anyhow::bail!(
                "已有相同生成任务 {}（{}）。读取该任务并复用结果或等待完成；明确重试时使用 regenerate_generation，不要重复创建。",
                text(&old["key"]),
                text(&old["status"])
            );
        }
    }
    ensure!(
        tasks(p)
            .into_iter()
            .filter(|t| t["turnId"] == context["turnId"])
            .count()
            < 500,
        "本批次最多创建 500 个生成任务，请在新一轮继续"
    );
    if run["targetNodeId"].is_null() && run["ownerId"].is_null() && !asset_task {
        run["position"] = layout::placement(p, &run, &turn["canvas"])?;
    }
    save(p, run);
    Ok(())
}
