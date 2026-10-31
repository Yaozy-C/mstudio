use super::*;
fn selectable(a: &Value) -> bool {
    text(&a["annotationSourceId"]).is_empty() && a["missing"] != true && a["inLibrary"] != false
}
fn script(script: &Value, args: &Value, full: bool) -> Value {
    let keys = [
        "title",
        "duration",
        "action",
        "onScreenText",
        "dialogue",
        "sound",
    ];
    let ids = &rows(&args["paragraphIds"])[..rows(&args["paragraphIds"]).len().min(12)];
    let fields = args["scriptFields"].as_array().cloned().unwrap_or_else(|| {
        if full {
            keys.iter().map(|k| json!(k)).collect()
        } else {
            vec![json!("title"), json!("duration")]
        }
    });
    let selected: Vec<_> = rows(script)
        .iter()
        .filter(|s| ids.is_empty() || ids.contains(&s["id"]))
        .collect();
    let offset = offset(&args["offset"]);
    let text_offset = super::offset(&args["textOffset"]);
    let items: Vec<_> = selected
        .iter()
        .skip(offset)
        .take(10)
        .map(|s| {
            let mut out = json!({"id":s["id"]});
            for k in keys {
                if fields.contains(&json!(k)) {
                    let default = if k == "duration" { json!(5) } else { json!("") };
                    let value = s.get(k).unwrap_or(&default);
                    out[k] = if value.is_number() || k == "title" {
                        value.clone()
                    } else {
                        snippet(value, text_offset, 1000)
                    };
                }
            }
            out
        })
        .collect();
    json!({"script":items,"scriptCount":rows(script).len(),"matchedScriptCount":selected.len(),"missingParagraphIds":ids.iter().filter(|id|!rows(script).iter().any(|s|&s["id"]==*id)).cloned().collect::<Vec<_>>(),"omittedScriptFields":keys.iter().filter(|k|!fields.contains(&json!(k))).collect::<Vec<_>>(),"nextScriptOffset":if selected.len()>offset+10{json!(offset+10)}else{Value::Null}})
}
pub(super) fn inspect(p: &Value, args: &Value) -> Result<Value> {
    let offset = offset(&args["offset"]);
    let text_offset = super::offset(&args["textOffset"]);
    let ids = &rows(&args["nodeIds"])[..rows(&args["nodeIds"]).len().min(12)];
    let explicit = args["fields"].is_array();
    let fields = args["fields"].as_array().cloned().unwrap_or(vec![
        json!("title"),
        json!("shot"),
        json!("script"),
        json!("assetId"),
        json!("resultAssetId"),
    ]);
    let wants = |k: &str| fields.contains(&json!(k));
    if args["section"].is_null()
        && ids.is_empty()
        && ((explicit && ["script", "screenplay", "shots"].iter().any(|k| wants(k)))
            || args.get("paragraphIds").is_some()
            || args.get("scriptFields").is_some())
    {
        anyhow::bail!(
            "Script reads require nodeIds containing the screenplay node ID; paragraphIds only filters paragraphs within that node. Use fields=[script] or [screenplay]."
        );
    }
    let detail_limit = if ids.len() == 1 || !text(&args["taskKey"]).is_empty() {
        4000
    } else {
        1000
    };
    let usage = args["usage"].as_str().unwrap_or("asset");
    ensure!(
        matches!(usage, "asset" | "material" | "all"),
        "素材用途筛选无效"
    );
    let direct = !rows(&args["ids"]).is_empty();
    let mut all_assets = rows(&p["assets"]).to_vec();
    if direct {
        all_assets.extend_from_slice(rows(&p["previsAssets"]));
    }
    let assets: Vec<_> = all_assets
        .iter()
        .filter(|a| {
            use crate::project_service::asset_roles as roles;
            selectable(a)
                && ((direct && args.get("usage").is_none())
                    || match usage {
                        "material" => roles::is_material(p, a),
                        "asset" => roles::is_asset(a) && (direct || roles::is_confirmed(a)),
                        _ => direct || roles::is_material(p, a) || roles::is_confirmed(a),
                    })
        })
        .map(|a| {
            let mut v = select(
                a,
                &[
                    "id",
                    "kind",
                    "duration",
                    "width",
                    "height",
                    "imageId",
                    "imageVersionId",
                    "imageVersionCount",
                    "inMaterials",
                    "assetStatus",
                ],
            );
            v["name"] = json!(clipped(&a["name"], 100));
            v["usage"] = json!(a["usage"].as_str().unwrap_or("material"));
            v["inMaterials"] = json!(crate::project_service::asset_roles::is_material(p, a));
            v["assetStatus"] = if crate::project_service::asset_roles::is_asset(a) {
                json!(if crate::project_service::asset_roles::is_confirmed(a) {
                    "confirmed"
                } else {
                    "candidate"
                })
            } else {
                Value::Null
            };
            v
        })
        .collect();
    let generation: Vec<_> = if args["section"] == "generation" {
        generation::tasks(p)
            .into_iter()
            .filter(|t| args["turnId"].is_null() || t["turnId"] == args["turnId"])
            .collect()
    } else {
        Vec::new()
    };
    if args["section"] == "creation" && ids.is_empty() {
        let mut creation = json!({"brief":snippet(&p["brief"],text_offset,800),"intent":snippet(p["creation"].get("intent").unwrap_or(&p["brief"]),text_offset,800),"essential":snippet(&p["creation"]["essential"],text_offset,800),"preserve":snippet(&p["creation"]["preserve"],text_offset,800)});
        if let Some(v) = p["creation"].get("stage") {
            creation["stage"] = v.clone();
        }
        return Ok(json!({"creation":creation,"section":"creation"}));
    }
    let mut page=match text(&args["section"]){"assets"=>Some(assets.clone()),"clips"=>Some(rows(&p["clips"]).to_vec()),"tracks"=>Some(rows(&p["tracks"]).to_vec()),"captions"=>Some(rows(&p["captions"]).iter().map(|c|{let mut v=c.clone();assign(&mut v,&snippet(&c["text"],text_offset,1000));v}).collect()),"concepts"=>Some(rows(&p["concepts"]).to_vec()),"generation"=>Some(generation.iter().filter(|t|(args["taskKey"].is_null()||args["taskKey"]==t["key"])&&(args["status"].is_null()||args["status"]==t["status"])).map(|t|{if t["hiddenFromList"]==true&&args["taskKey"].is_null(){let mut v=select(t,&["kind","targetNodeId","status"]);assign(&mut v,&json!({"id":t["key"],"title":title(t),"archived":true,"hiddenFromList":true,"resultAssetIds":generation::results(t).into_iter().filter(|id|rows(&p["assets"]).iter().any(|a|&a["id"]==id)).collect::<Vec<_>>(),"continuation":generation::outcome(t)["continuation"]}));if t.get("instruction").is_some(){v["summary"]=json!(clipped(&t["instruction"],120));}v}else{let mut v=select(t,&["turnId","kind","generationPurpose","ownerId","targetNodeId","trackingPaused","status","resultAssetId","resultAssetIds","modelId","inputs","parameters","error"]);assign(&mut v,&json!({"id":t["key"],"title":title(t),"hiddenFromList":t["hiddenFromList"]==true,"continuation":generation::outcome(t)["continuation"]}));if wants("prompt"){v["prompt"]=snippet(&t["prompt"],text_offset,detail_limit);}v}}).collect()),_=>None};
    if let Some(page) = &mut page {
        let object_ids = &rows(&args["ids"])[..rows(&args["ids"]).len().min(12)];
        let missing: Vec<_> = object_ids
            .iter()
            .filter(|id| !page.iter().any(|v| &v["id"] == *id))
            .cloned()
            .collect();
        if !object_ids.is_empty() {
            page.retain(|v| object_ids.contains(&v["id"]));
        }
        if explicit {
            for item in page.iter_mut() {
                if item["archived"] == true {
                    continue;
                }
                let vocabulary: Vec<String> = if args["section"] == "assets" {
                    [
                        "id",
                        "name",
                        "usage",
                        "kind",
                        "duration",
                        "width",
                        "height",
                        "imageId",
                        "imageVersionId",
                        "imageVersionCount",
                        "inMaterials",
                        "assetStatus",
                    ]
                    .iter()
                    .map(|k| k.to_string())
                    .collect()
                } else {
                    item.as_object()
                        .into_iter()
                        .flat_map(|m| m.keys())
                        .cloned()
                        .collect()
                };
                let omitted: Vec<_> = vocabulary
                    .into_iter()
                    .filter(|k| {
                        k.as_str() != "id"
                            && !wants(k)
                            && !(args["section"] == "generation"
                                && matches!(k.as_str(), "status" | "continuation"))
                    })
                    .collect();
                for k in &omitted {
                    obj(item).remove(k);
                }
                if args["section"] != "generation" {
                    item["omittedFields"] = json!(omitted);
                }
            }
        }
        let limit = if args["section"] == "generation" {
            if explicit { 30 } else { 5 }
        } else {
            12
        };
        let mut selected = Vec::new();
        let mut bytes = 0;
        for item in page.iter().skip(offset).take(limit) {
            let n = item.to_string().len();
            if !selected.is_empty() && bytes + n > 12000 {
                break;
            }
            bytes += n;
            selected.push(item.clone());
        }
        let mut out = json!({"section":args["section"],"missingIds":missing,"total":page.len(),"nextOffset":if offset+selected.len()<page.len(){json!(offset+selected.len())}else{Value::Null},"items":selected});
        if args["section"] == "generation" {
            out["batch"] = generation::batch(
                &generation
                    .iter()
                    .copied()
                    .filter(|t| t["hiddenFromList"] != true)
                    .collect::<Vec<_>>(),
            );
        }
        return Ok(out);
    }
    let mut details = Vec::new();
    for n in rows(&p["nodes"]).iter().filter(|n| ids.contains(&n["id"])) {
        let mut out = select(n, &["id", "kind"]);
        if wants("title") {
            out["title"] = json!(clipped(&n["title"], 80));
        }
        if wants("text") {
            assign(&mut out, &snippet(&n["text"], text_offset, detail_limit));
        }
        for k in ["assetId", "resultAssetId"] {
            if wants(k)
                && let Some(v) = n.get(k)
            {
                out[k] = v.clone();
            }
        }
        out["omittedFields"] = json!(
            ["title", "text", "shot", "screenplay", "references"]
                .iter()
                .filter(|k| !wants(k))
                .collect::<Vec<_>>()
        );
        if n["screenplay"].is_object() && (wants("screenplay") || wants("script")) {
            out["screenplay"] = script(&n["screenplay"]["script"], args, wants("screenplay"));
        }
        if n["shot"].is_object() {
            let shot = &n["shot"];
            let mut s = json!({});
            if wants("shot") {
                s = select(shot, &["screenplayId", "scriptId", "visualChanged"]);
                s["scriptChanged"] = json!(creative::script_changed(p, n));
            }
            for k in ["order", "duration"] {
                if (wants("shot")
                    || wants(&format!("shot.{k}"))
                    || (k == "duration" && wants("duration")))
                    && let Some(v) = shot.get(k)
                {
                    s[k] = v.clone();
                }
            }
            if wants("frames") {
                s["frames"] = json!(
                    rows(&shot["frames"])
                        .iter()
                        .skip(offset)
                        .take(5)
                        .map(|f| select(f, &["assetId", "title"]))
                        .collect::<Vec<_>>()
                );
            }
            for k in ["framePrompt", "dialogue", "prompt"] {
                if wants(k) {
                    s[k] = snippet(
                        &shot[k],
                        text_offset,
                        if k == "dialogue" { 1000 } else { detail_limit },
                    );
                }
            }
            if wants("shot") || wants("prompt") {
                s["promptStale"] = json!(
                    !text(&shot["prompt"]).is_empty()
                        && shot["promptBasis"] != creative::prompt_basis(p, n)
                );
            }
            if wants("takes") && shot.get("takes").is_some() {
                s["takes"] = json!(
                    rows(&shot["takes"])
                        .iter()
                        .skip(offset)
                        .take(5)
                        .map(|t| {
                            let mut t = t.clone();
                            obj(&mut t).remove("basis");
                            obj(&mut t).remove("production");
                            t
                        })
                        .collect::<Vec<_>>()
                );
            }
            out["shot"] = s;
        }
        let mut shots: Vec<_> = rows(&p["nodes"])
            .iter()
            .filter(|s| s["shot"]["screenplayId"] == n["id"])
            .collect();
        shots.sort_by(|a, b| num(&a["shot"]["order"], 0.).total_cmp(&num(&b["shot"]["order"], 0.)));
        if wants("shots") && n["kind"] == "screenplay" {
            out["shots"]=json!(shots.iter().skip(offset).take(10).map(|s|json!({"id":s["id"],"title":s["title"],"order":s["shot"]["order"],"duration":s["shot"]["duration"]})).collect::<Vec<_>>());
        }
        out["nextShotOffset"] =
            if wants("shots") && n["kind"] == "screenplay" && shots.len() > offset + 10 {
                json!(offset + 10)
            } else {
                Value::Null
            };
        if wants("references") && n.get("references").is_some() {
            out["references"] = json!(
                rows(&n["references"])
                    .iter()
                    .skip(offset)
                    .take(4)
                    .map(|r| {
                        let mut r = r.clone();
                        r["purpose"] = json!(clipped(&r["purpose"], 400));
                        r
                    })
                    .collect::<Vec<_>>()
            );
        }
        out["nextReferenceOffset"] =
            if wants("references") && rows(&n["references"]).len() > offset + 4 {
                json!(offset + 4)
            } else {
                Value::Null
            };
        details.push(out);
    }
    let directory: Vec<_> = if ids.is_empty() {
        rows(&p["nodes"])
            .iter()
            .skip(offset)
            .take(10)
            .map(|n| {
                let mut v = select(n, &["id", "kind", "assetId", "resultAssetId"]);
                v["title"] = json!(clipped(&n["title"], 80));
                for (k, path) in [("screenplayId", "screenplayId"), ("order", "order")] {
                    if let Some(x) = n["shot"].get(path) {
                        v[k] = x.clone();
                    }
                }
                v
            })
            .collect()
    } else {
        Vec::new()
    };
    let mut out = json!({"name":clipped(&p["name"],200),"format":select(p,&["width","height","fps"]),"nodeCount":rows(&p["nodes"]).len(),"nextOffset":if ids.is_empty()&&offset+10<rows(&p["nodes"]).len(){json!(offset+10)}else{Value::Null},"nodes":directory,"details":details,"missingNodeIds":ids.iter().filter(|id|node(p,id).is_none()).cloned().collect::<Vec<_>>(),"assets":if ids.is_empty(){assets.iter().skip(offset).take(10).cloned().collect::<Vec<_>>()}else{vec![]},"assetCount":assets.len(),"clips":if ids.is_empty(){rows(&p["clips"]).iter().skip(offset).take(10).cloned().collect::<Vec<_>>()}else{vec![]},"clipCount":rows(&p["clips"]).len(),"tracks":if ids.is_empty(){rows(&p["tracks"]).iter().take(10).cloned().collect::<Vec<_>>()}else{vec![]},"captionCount":rows(&p["captions"]).len()});
    out["materialCount"] = json!(
        rows(&p["assets"])
            .iter()
            .filter(|a| selectable(a) && crate::project_service::asset_roles::is_material(p, a))
            .count()
    );
    out["candidateAssetCount"] = json!(
        rows(&p["assets"])
            .iter()
            .filter(|a| crate::project_service::asset_roles::is_asset(a)
                && !crate::project_service::asset_roles::is_confirmed(a))
            .count()
    );
    if !rows(&p["concepts"]).is_empty() {
        out["conceptCount"] = json!(rows(&p["concepts"]).len());
        out["concepts"] = json!(rows(&p["concepts"])
            .iter()
            .map(|c| json!({"id":c["id"],"title":c["title"],"oneLine":c["oneLine"],"status":c["status"]}))
            .collect::<Vec<_>>());
    }
    if ids.is_empty() {
        out["brief"] = json!(clipped(&p["brief"], 500));
    } else {
        out["readDetails"] = json!(
            "Use fields for full text; script with paragraphIds/scriptFields narrows paragraphs; screenplay returns the full paginated document."
        );
    }
    if p["creation"].is_object() && (ids.is_empty() || args["section"] == "creation") {
        let c = &p["creation"];
        let mut v = select(c, &["stage"]);
        let mut truncated = false;
        for k in ["intent", "essential", "preserve"] {
            v[k] = json!(clipped(&c[k], 300));
            truncated |= text(&c[k]).encode_utf16().count() > 300;
        }
        v["truncated"] = json!(truncated);
        out["creation"] = v;
    }
    Ok(out)
}
