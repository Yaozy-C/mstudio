use super::*;

/// Validates one JSON Pointer. The request encoder resolves the same shape.
pub fn pointer_is_valid(path: &str) -> bool {
    if !path.starts_with('/') || path.len() > 300 {
        return false;
    }
    path[1..].split('/').all(|segment| {
        let decoded = segment.replace("~1", "/").replace("~0", "~");
        if ["__proto__", "constructor", "prototype"].contains(&decoded.as_str()) {
            return false;
        }
        let mut chars = segment.chars();
        while let Some(character) = chars.next() {
            if character == '~' && !matches!(chars.next(), Some('0' | '1')) {
                return false;
            }
        }
        true
    })
}

fn validate_control(name: &str, control: &ControlDeclaration) -> Result<()> {
    ensure!(
        pointer_is_valid(&control.path),
        "参数控件 {name} 的 path 必须是 JSON Pointer，例如 /duration"
    );
    if let Some(values) = &control.values {
        ensure!(!values.is_empty(), "参数控件 {name} 的 values 数量无效");
        ensure!(
            values.iter().all(|value| !value.trim().is_empty()),
            "参数控件 {name} 的 values 不能为空字符串"
        );
    }
    ensure!(
        [control.min, control.max]
            .into_iter()
            .flatten()
            .all(|v| v.is_finite() && v >= 0.),
        "参数控件 {name} 的范围须为非负有限数字"
    );
    if let (Some(min), Some(max)) = (control.min, control.max) {
        ensure!(min <= max, "参数控件 {name} 的 min 不能大于 max");
    }
    match name {
        // Numeric controls are bounded by min/max; enum controls need their values.
        "duration" | "imageSize" => {
            ensure!(
                control.values.is_none(),
                "参数控件 {name} 使用 path 与 min/max"
            )
        }
        _ => ensure!(control.values.is_some(), "参数控件 {name} 需要 values 枚举"),
    }
    Ok(())
}

pub fn validate(model: &MediaModel) -> Result<()> {
    let Some(capabilities) = &model.capabilities else {
        return Ok(());
    };
    if let Some(references) = &capabilities.references {
        let mut keys = std::collections::HashSet::new();
        for reference in references {
            ensure!(
                pointer_is_valid(&reference.key),
                "参考字段 key 必须是 JSON Pointer，例如 /image_url"
            );
            ensure!(
                keys.insert(&reference.key),
                "参考字段重复：{}",
                reference.key
            );
            ensure!(
                KINDS.contains(&reference.kind.as_str()),
                "参考类型无效：{}",
                reference.kind
            );
            ensure!(
                ROLES.contains(&reference.role.as_str()),
                "参考用途无效：{}",
                reference.role
            );
            ensure!(
                reference.max.is_none_or(|max| max >= 1),
                "参考上限 max 必须是正整数"
            );
        }
    }
    if let Some(controls) = &capabilities.controls {
        for (name, control) in [
            ("aspectRatio", &controls.aspect_ratio),
            ("resolution", &controls.resolution),
            ("duration", &controls.duration),
            ("imageSize", &controls.image_size),
        ] {
            if let Some(control) = control {
                validate_control(name, control)?;
            }
        }
    }
    ensure!(
        capabilities.reference_limit.is_none_or(|limit| limit > 0),
        "参考输入总数上限须为正整数"
    );
    ensure!(
        capabilities
            .reference_seconds
            .is_none_or(|seconds| seconds.is_finite() && seconds > 0.),
        "参考视频总时长上限须为有限正数"
    );
    // A protocol that posts a fixed template has nowhere to put a reference, and the
    // Gemini and Codex builders can only embed images.
    let declared = capabilities.references.is_some();
    if declared {
        ensure!(
            model.plugin != "http-json",
            "自定义 HTTP 的参考素材写在请求模板里，不能声明参考字段"
        );
        if ["gemini-native", "codex-image"].contains(&model.plugin.as_str()) {
            ensure!(
                effective_references(model)
                    .iter()
                    .all(|reference| reference.kind == "image"),
                "此连接的请求格式只支持图片参考"
            );
        }
    }
    if let Some(references) = &capabilities.references
        && ["gemini-native", "codex-image"].contains(&model.plugin.as_str())
    {
        let key = if model.plugin == "gemini-native" {
            "/image_urls"
        } else {
            "/image"
        };
        ensure!(
            references
                .iter()
                .all(|r| r.key == key && r.role == "reference" && r.multiple),
            "此连接的图片参考必须使用固定字段 {key}、reference 用途和数组格式"
        );
    }
    if let Some(controls) = &capabilities.controls {
        if model.plugin == "codex-image" {
            ensure!(
                *controls == Controls::default(),
                "Codex 不支持声明生成参数控件"
            );
        }
        if model.plugin == "gemini-native" {
            ensure!(
                controls.duration.is_none() && controls.image_size.is_none(),
                "Gemini 生图不支持时长或自定义尺寸控件"
            );
            for (control, path) in [
                (
                    &controls.aspect_ratio,
                    "/generationConfig/imageConfig/aspectRatio",
                ),
                (
                    &controls.resolution,
                    "/generationConfig/imageConfig/imageSize",
                ),
            ] {
                ensure!(
                    control.as_ref().is_none_or(|c| c.path == path),
                    "Gemini 参数控件必须使用固定字段 {path}"
                );
            }
        }
    }
    Ok(())
}
