use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::path::Path;

pub fn validate(input: &Value) -> Result<()> {
    let object = input.as_object().context("请求必须是 JSON 对象")?;
    for key in object.keys() {
        ensure!(
            [
                "prompt",
                "model",
                "n",
                "size",
                "quality",
                "output_format",
                "image"
            ]
            .contains(&key.as_str()),
            "Codex 不支持参数：{key}"
        );
    }
    let prompt = input["prompt"].as_str().context("请输入生成描述")?;
    ensure!(
        !prompt.trim().is_empty() && prompt.len() <= 32000,
        "生成描述须为 1–32000 字节"
    );
    for (key, expected) in [
        ("model", "codex-image"),
        ("size", "auto"),
        ("quality", "auto"),
        ("output_format", "png"),
    ] {
        ensure!(
            input.get(key).is_none_or(|v| v == expected),
            "Codex 不支持指定 {key}；当前只支持 {expected}，不能保证 Image 2.5 型号或精确参数"
        );
    }
    // Legacy model presets may contain n; native tool calls determine outputs.
    if let Some(images) = input.get("image") {
        let images = images.as_array().context("image 必须为内嵌图片数组")?;
        ensure!(!images.is_empty(), "编辑至少需要 1 张参考图片");
        let mut total = 0;
        for image in images {
            let url = image.as_str().context("参考图片格式无效")?;
            total += url.len();
            ensure!(total <= 25_000_000, "参考图片总量超过限制");
            super::image_data::image_bytes(url)?;
        }
    }
    Ok(())
}
pub fn turn_input(input: &Value, root: &Path) -> Result<Value> {
    validate(input)?;
    let mut references = Vec::new();
    for (i, image) in input["image"].as_array().into_iter().flatten().enumerate() {
        let (mime, bytes) = super::image_data::image_bytes(image.as_str().unwrap())?;
        let extension = match mime {
            "image/jpeg" => "jpg",
            "image/webp" => "webp",
            _ => "png",
        };
        let path = root.join(format!("reference-{i}.{extension}"));
        std::fs::write(&path, bytes)?;
        references.push(json!({"type":"localImage","path":path}));
    }
    let paths: Vec<_> = references.iter().map(|item| item["path"].clone()).collect();
    let prompt = format!(
        "Use the native image_gen.imagegen tool to fulfill the user request. Call it as many times as needed for the requested images. Return separate images when requested. Do not use API keys, HTTP clients, other providers, shell drawing, or SVG. If the tool is unavailable, report the error. The user image description follows:\n{}",
        input["prompt"].as_str().unwrap()
    );
    let prompt = if paths.is_empty() {
        prompt
    } else {
        format!(
            "{prompt}\nReference image paths: {}. Include all these paths in referenced_image_paths when calling image_gen.imagegen; do not use num_last_images_to_include or drop references.",
            json!(paths)
        )
    };
    let mut items = vec![json!({"type":"text","text":prompt})];
    items.extend(references);
    Ok(json!(items))
}
pub fn image_result(item: &Value) -> Result<Option<Value>> {
    if item["type"] != "imageGeneration" {
        return Ok(None);
    }
    // App Server already returns native tool failures to the model.
    // This event is only a notification; keep listening for its next action.
    if !item["failure"].is_null() || matches!(item["status"].as_str(), Some("failed" | "cancelled"))
    {
        return Ok(None);
    }
    ensure!(
        item["status"] == "completed",
        "Codex 生图未完成：{}",
        item["status"]
    );
    let encoded = item["result"]
        .as_str()
        .filter(|v| !v.is_empty())
        .context("Codex 未返回图片数据")?;
    let (_, bytes) = super::image_data::image_bytes(&format!("data:image/png;base64,{encoded}"))?;
    ensure!(
        bytes.starts_with(b"\x89PNG\r\n\x1a\n"),
        "Codex 返回了非 PNG 图片"
    );
    Ok(Some(json!({"b64_json":encoded})))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn all_references_are_forwarded_by_path_without_a_five_image_cap() {
        let stamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("codex-refs-{}-{stamp}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let input =
            json!({"prompt":"Use every reference", "image":vec!["data:image/png;base64,YQ==";12]});
        let items = turn_input(&input, &root).unwrap();
        assert_eq!(items.as_array().unwrap().len(), 13);
        assert!(
            items[0]["text"]
                .as_str()
                .unwrap()
                .contains("reference-11.png")
        );
        assert!(
            items[0]["text"]
                .as_str()
                .unwrap()
                .contains("referenced_image_paths")
        );
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn rejects_unsupported_controls_instead_of_silently_ignoring_them() {
        assert!(validate(&json!({"prompt":"hello","model":"codex-image","n":1})).is_ok());
        for input in [
            json!({"prompt":"hello","model":"gpt-image-2.5-sunburst"}),
            json!({"prompt":"hello","size":"1024x1024"}),
            json!({"prompt":"hello","mask":"x"}),
            json!({"prompt":"hello","image":[]}),
        ] {
            assert!(validate(&input).is_err());
        }
    }
    #[test]
    fn image_failure_is_not_a_successful_generation() {
        assert!(
            image_result(&json!({"type":"imageGeneration","status":"completed","result":""}))
                .is_err()
        );
        assert!(
            image_result(
                &json!({"type":"imageGeneration","failure":{"type":"usageLimitExceeded"}})
            )
            .unwrap()
            .is_none()
        );
        assert!(
            image_result(&json!({"type":"agentMessage","text":"done"}))
                .unwrap()
                .is_none()
        );
    }
}
