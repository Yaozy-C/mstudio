//! Generation submission assembles model parameters and media inputs natively.
use super::generation_parameters::{remove, resolve, set};
use crate::{
    models::{capabilities, media::MediaModel},
    previs::media::find,
};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

fn items(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}

fn role(input: &Value) -> &str {
    input["role"].as_str().unwrap_or("")
}

fn label(input: &Value) -> &'static str {
    match role(input) {
        "edit" => "要修改的图",
        "first-frame" => "首帧",
        "last-frame" => "尾帧",
        "video-reference" => "动作参考",
        "video-edit" => "修改原视频",
        _ => "内容参考",
    }
}

fn reference(input: &Value) -> Value {
    let label = label(input);
    let purpose = input["purpose"].as_str().unwrap_or("").trim();
    let purpose = if purpose.is_empty() {
        label.to_owned()
    } else if purpose == label || purpose.starts_with(&format!("{label} · ")) {
        purpose.to_owned()
    } else {
        format!("{label} · {purpose}")
    };
    let mut result = json!({"assetId":input["assetId"],"purpose":purpose});
    for field in ["start", "end"] {
        if let Some(value) = input.get(field) {
            result[field] = value.clone();
        }
    }
    result
}

fn valid_url(url: &str, kind: &str, plugin: &str) -> bool {
    if let Some(data) = url.strip_prefix(&format!("data:{kind}/"))
        && let Some((mime, _)) = data.split_once(";base64,")
    {
        let valid_mime = !mime.is_empty()
            && mime
                .bytes()
                .all(|b| b.is_ascii_alphanumeric() || b".+-".contains(&b));
        return valid_mime
            && (!["codex-image", "gemini-native"].contains(&plugin)
                || (kind == "image" && ["png", "jpeg", "webp"].contains(&mime)));
    }
    if ["codex-image", "gemini-native"].contains(&plugin) {
        return false;
    }
    reqwest::Url::parse(url).is_ok_and(|u| {
        (matches!(u.scheme(), "http" | "https") || (plugin == "dashscope" && u.scheme() == "oss"))
            && u.username().is_empty()
            && u.password().is_none()
    })
}

fn replace_prompt(value: &mut Value, prompt: &str) {
    match value {
        Value::String(s) => *s = s.replace("{{prompt}}", prompt),
        Value::Array(values) => values.iter_mut().for_each(|v| replace_prompt(v, prompt)),
        Value::Object(values) => values.values_mut().for_each(|v| replace_prompt(v, prompt)),
        _ => {}
    }
}

pub(super) fn prepare(
    document: &Value,
    key: &str,
    model: &Value,
    uploaded: Option<Value>,
) -> Result<Value> {
    let model: MediaModel = serde_json::from_value(model.clone())?;
    crate::models::media::validate(&model)?;
    let task = document["production"]["drafts"]
        .get(key)
        .context("Generation task no longer exists")?;
    ensure!(
        task["hiddenFromList"] != true,
        "请先恢复已归档任务，再开始生成"
    );
    ensure!(
        model.enabled && task["kind"] == model.kind,
        "请选择对应类型的已启用模型"
    );
    let prompt = task["prompt"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .context("先说说这次想生成或修改什么")?
        .trim();
    let visual: Vec<_> = items(&task["inputs"])
        .filter(|r| role(r) != "script")
        .collect();
    ensure!(
        model.kind != "video" || !visual.iter().any(|r| role(r) == "edit"),
        "请将修改图片改为视频参考或明确指定首帧"
    );
    ensure!(
        model.kind != "image"
            || !visual
                .iter()
                .any(|r| matches!(role(r), "first-frame" | "last-frame")),
        "图片生成不能使用首尾帧，请将用途改为内容参考"
    );
    ensure!(
        !visual.iter().any(|r| role(r) == "video-edit"),
        "当前接入的模型暂不支持直接修改原视频；可将视频设为动作参考，重新生成"
    );
    if model.kind == "video" && task["mode"] == "ends" {
        ensure!(
            visual.iter().filter(|r| role(r) == "first-frame").count() == 1
                && visual.iter().filter(|r| role(r) == "last-frame").count() == 1,
            "首尾帧需要分别指定一张首帧和一张尾帧"
        );
    }
    let source = items(&document["nodes"])
        .find(|n| n["id"] == task["ownerId"] && task["ownerId"].is_string());
    if task["ownerId"].as_str().is_some_and(|s| !s.is_empty()) {
        ensure!(
            source.is_some_and(|n| n["shot"].is_object()),
            "目标镜头已移除，请重新选择"
        );
    }
    for input in visual.iter().filter(|r| r["frame"] == true) {
        ensure!(
            items(&document["nodes"]).any(|n| n["id"] == input["nodeId"]
                && items(&n["shot"]["frames"]).any(|f| f["assetId"] == input["assetId"]))
                && items(&document["assets"])
                    .any(|a| a["id"] == input["assetId"] && a["kind"] == "image"),
            "所选分镜图已移除"
        );
    }
    let fields = capabilities::effective_references(&model);
    let mut assets = Vec::new();
    let mut seconds = Vec::new();
    for input in &visual {
        let asset = find(document, input["assetId"].as_str().unwrap_or(""))
            .context("参考素材已移除或不可用，请替换该参考")?;
        ensure!(
            asset["missing"] != true,
            "参考素材已移除或不可用，请替换该参考"
        );
        ensure!(
            !asset["annotationSourceId"]
                .as_str()
                .is_some_and(|s| !s.is_empty()),
            "内部批注图不能用作生成参考，请使用原图"
        );
        ensure!(
            asset["inLibrary"] != false,
            "参考素材已从素材库移除，请移除该参考或先恢复素材"
        );
        ensure!(
            task["generationPurpose"] == "asset"
                || (task["ownerId"].is_null() && task["targetNodeId"].is_null())
                || super::asset_roles::is_production_reference(document, asset),
            super::asset_roles::MATERIAL_REFERENCE_ERROR
        );
        ensure!(
            matches!(asset["kind"].as_str(), Some("image" | "video" | "audio")),
            "素材已移除，或不支持用于媒体生成"
        );
        let input_role = if matches!(role(input), "first-frame" | "last-frame") {
            role(input)
        } else {
            "reference"
        };
        ensure!(
            fields
                .iter()
                .any(|f| f.kind == asset["kind"] && f.role == input_role),
            "此模型不支持「{}」的{}输入，请更换模型或素材用途",
            label(input),
            if asset["kind"] == "video" {
                "视频"
            } else {
                "图片"
            }
        );
        let mut duration = 0.;
        if asset["kind"] == "video" {
            let total = asset["duration"].as_f64().context("参考视频区间无效")?;
            let start = input["start"].as_f64().unwrap_or(0.);
            let end = input["end"].as_f64().unwrap_or(total);
            ensure!(
                start >= 0. && end <= total && end > start,
                "参考区间须有效且不能超出原视频"
            );
            duration = end - start;
        }
        seconds.push(duration);
        assets.push((asset, input_role));
    }
    let videos: Vec<_> = assets
        .iter()
        .enumerate()
        .filter(|(_, (a, _))| a["kind"] == "video")
        .map(|(i, _)| i)
        .collect();
    let merge = videos.len() > 1 && videos.iter().any(|&i| seconds[i] < 2.);
    let mut groups = Vec::new();
    for i in 0..visual.len() {
        if merge && videos.contains(&i) {
            if i == videos[0] {
                groups.push(videos.clone());
            }
        } else {
            groups.push(vec![i]);
        }
    }
    let limits = capabilities::limits(&model);
    ensure!(
        limits.references.is_none_or(|n| groups.len() <= n),
        "本次最多使用 {} 个素材",
        limits.references.unwrap_or(0)
    );
    ensure!(
        limits
            .seconds
            .is_none_or(|n| seconds.iter().sum::<f64>() <= n),
        "参考视频合计不超过 {} 秒",
        limits.seconds.unwrap_or(0.)
    );
    let references: Vec<_> = visual.iter().map(|r| reference(r)).collect();
    let mut input = resolve(&model, task, document)?;
    for field in &fields {
        remove(&mut input, &field.key);
    }
    for (plugin, field) in [("codex-image", "/image"), ("gemini-native", "/image_urls")] {
        if model.plugin == plugin {
            remove(&mut input, field);
        }
    }
    let mut images = 0;
    let mut videos = 0;
    let mut audios = 0;
    let mut descriptions = Vec::new();
    let mut files = Vec::new();
    for group in &groups {
        let i = group[0];
        let (asset, input_role) = assets[i];
        let file_for = |index: usize| {
            items(uploaded.as_ref().unwrap_or(&Value::Null)).find(|f| {
                f["assetId"] == visual[index]["assetId"]
                    || items(&f["sourceAssetIds"]).any(|id| *id == visual[index]["assetId"])
            })
        };
        let file = file_for(i);
        if uploaded.is_some() {
            ensure!(
                group.iter().all(|&index| file_for(index)
                    .is_some_and(|f| f["kind"] == assets[index].0["kind"]
                        && f["url"].is_string()
                        && f["url"] == file.unwrap_or(&Value::Null)["url"])),
                "参考素材上传不完整，请重试"
            );
        }
        let url = file.and_then(|f| f["url"].as_str()).unwrap_or(
            if ["codex-image", "gemini-native"].contains(&model.plugin.as_str()) {
                "data:image/png;base64,YQ=="
            } else {
                "https://example.invalid/validate"
            },
        );
        ensure!(
            valid_url(url, asset["kind"].as_str().unwrap_or(""), &model.plugin),
            "参考媒体需要有效 URL 或 Base64 数据"
        );
        files.push((asset["kind"].as_str().unwrap(), input_role, url));
        let name = if asset["kind"] == "image" {
            images += 1;
            format!("Image {images}")
        } else if asset["kind"] == "video" {
            videos += 1;
            format!("Video {videos}")
        } else {
            audios += 1;
            format!("Audio {audios}")
        };
        let mut offset = 0.;
        for &index in group {
            let range = if group.len() > 1 {
                format!(" ({:.3}–{:.3}s)", offset, offset + seconds[index])
            } else {
                String::new()
            };
            offset += seconds[index];
            descriptions.push(format!(
                "{name}{range}: {}",
                references[index]["purpose"].as_str().unwrap()
            ));
        }
    }
    let prompt = std::iter::once(prompt.to_owned())
        .chain(descriptions)
        .collect::<Vec<_>>()
        .join("\n\n");
    if model.plugin == "http-json" {
        ensure!(
            files.is_empty(),
            "自定义 HTTP 请在请求模板中配置服务可访问的参考 URL"
        );
        replace_prompt(&mut input, &prompt);
    } else {
        input["prompt"] = json!(prompt);
        for field in fields {
            let values: Vec<_> = files
                .iter()
                .filter(|(kind, role, _)| *kind == field.kind && *role == field.role)
                .map(|(_, _, url)| json!(url))
                .collect();
            if !values.is_empty() {
                ensure!(
                    field.multiple || values.len() == 1,
                    "此输入角色只允许一个文件"
                );
                set(
                    &mut input,
                    &field.key,
                    if field.multiple {
                        json!(values)
                    } else {
                        values.into_iter().next().unwrap()
                    },
                )?;
            }
        }
    }
    capabilities::validate_input(&model, &input)?;
    // Runs reserve a display position when created. For older drafts, the
    // viewport supplies an origin; result receipt handles card collisions.
    let position = task
        .get("position")
        .cloned()
        .unwrap_or_else(|| legacy_position(document));
    let mut shot = source.cloned().unwrap_or(json!({}));
    shot["title"] = json!(
        task["title"]
            .as_str()
            .map(str::trim)
            .filter(|s| !s.is_empty())
            .unwrap_or("未命名")
    );
    shot["references"] = json!(references);
    shot["canvasGeneration"] = json!({"task":task,"x":position["x"],"y":position["y"]});
    if let Some(id) = task.get("submissionId") {
        shot["submissionId"] = id.clone();
    }
    if let Some(revision) = document.get("revision") {
        shot["projectRevision"] = revision.clone();
    }
    Ok(json!({"references":references,"input":input,"shot":shot}))
}

fn legacy_position(document: &Value) -> Value {
    let view = document["production"]
        .get("viewport")
        .unwrap_or(&document["viewport"]);
    let scale = view["scale"].as_f64().filter(|s| *s > 0.).unwrap_or(1.);
    let x = (32. - view["x"].as_f64().unwrap_or(0.)) / scale;
    let y = (80. - view["y"].as_f64().unwrap_or(0.)) / scale;
    json!({"x":x,"y":y})
}

#[cfg(test)]
#[path = "generation_prepare_tests.rs"]
mod tests;
