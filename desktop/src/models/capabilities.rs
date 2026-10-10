//! Model-declared reference inputs and generation controls.
//!
//! A protocol adapter defines how bytes travel; what a model accepts is a model fact, so it
//! is stored with the model and validated here. The declaration is data, never code: field
//! paths are JSON Pointers checked against the request body vocabulary, roles and kinds
//! come from fixed lists, and limits can only lower the system ceiling.
use super::media::MediaModel;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

const KINDS: [&str; 3] = ["image", "video", "audio"];
const ROLES: [&str; 4] = ["reference", "first-frame", "last-frame", "mask"];
const MAX_REFERENCE_ENTRIES: usize = 24;
const MAX_VALUES: usize = 64;
/// System ceilings. A declaration may lower them, never raise them.
pub const MAX_REFERENCES: usize = 12;
pub const MAX_REFERENCE_SECONDS: f64 = 15.0;
pub const MAX_IMAGES: usize = 9;
pub const MAX_VIDEOS: usize = 3;
/// Marks the one-time stamping of declarations onto models saved before this schema.
const MIGRATION_KEY: &str = "media-capabilities-v1";

fn is_false(value: &bool) -> bool {
    !*value
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ReferenceDeclaration {
    pub key: String,
    pub kind: String,
    pub role: String,
    #[serde(default, skip_serializing_if = "is_false")]
    pub multiple: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    pub required: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ControlDeclaration {
    pub path: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<String>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub min: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<u32>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Controls {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub aspect_ratio: Option<ControlDeclaration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub resolution: Option<ControlDeclaration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub duration: Option<ControlDeclaration>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub image_size: Option<ControlDeclaration>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capabilities {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<ReferenceDeclaration>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controls: Option<Controls>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_seconds: Option<f64>,
}

/// Validates one JSON Pointer. The request encoder resolves the same shape.
pub fn pointer_is_valid(path: &str) -> bool {
    if !path.starts_with('/') || path.len() > 300 {
        return false;
    }
    path[1..].split('/').all(|segment| {
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
        ensure!(
            !values.is_empty() && values.len() <= MAX_VALUES,
            "参数控件 {name} 的 values 数量无效"
        );
        ensure!(
            values.iter().all(|value| !value.trim().is_empty()),
            "参数控件 {name} 的 values 不能为空字符串"
        );
    }
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

/// The declaration a model actually uses, including the vocabulary its protocol fixes.
pub fn effective_references(model: &MediaModel) -> Vec<ReferenceDeclaration> {
    if let Some(declared) = model
        .capabilities
        .as_ref()
        .and_then(|capabilities| capabilities.references.as_ref())
    {
        return declared.clone();
    }
    match model.plugin.as_str() {
        // These request builders embed references themselves, so every model on the
        // protocol accepts the same image references.
        "gemini-native" => vec![ReferenceDeclaration {
            key: "/image_urls".into(),
            kind: "image".into(),
            role: "reference".into(),
            multiple: true,
            required: false,
            max: Some(MAX_IMAGES as u32),
        }],
        "codex-image" => vec![ReferenceDeclaration {
            key: "/image".into(),
            kind: "image".into(),
            role: "reference".into(),
            multiple: true,
            required: false,
            max: None,
        }],
        _ => Vec::new(),
    }
}

/// The total ceilings for one request, defaulting to the system caps.
///
/// Per-kind ceilings stay at the system level: a role-level `max` (first frame, last frame)
/// must not be read as a whole-kind cap, or a first-and-last-frame request would be
/// rejected. Role limits are enforced per declared field by `validate_input`.
pub struct Limits {
    pub references: usize,
    pub seconds: f64,
}

pub fn limits(model: &MediaModel) -> Limits {
    let declared = model.capabilities.as_ref();
    Limits {
        references: declared
            .and_then(|capabilities| capabilities.reference_limit)
            .map_or(MAX_REFERENCES, |limit| limit as usize),
        seconds: declared
            .and_then(|capabilities| capabilities.reference_seconds)
            .unwrap_or(MAX_REFERENCE_SECONDS),
    }
}

/// Enforces the declared reference vocabulary against the request body about to be sent.
///
/// The front end encodes references at the declared pointers; this is the last gate before
/// a paid call, so a mismatch fails here instead of at the provider.
pub fn validate_input(model: &MediaModel, input: &serde_json::Value) -> Result<()> {
    let mut total = 0;
    for reference in effective_references(model) {
        let count = match input.pointer(&reference.key) {
            None | Some(serde_json::Value::Null) => 0,
            Some(serde_json::Value::Array(items)) => items.len(),
            Some(_) => 1,
        };
        if reference.required {
            ensure!(count > 0, "缺少必填的参考输入：{}", reference.key);
        }
        if let Some(max) = reference.max {
            ensure!(
                count <= max as usize,
                "参考输入 {} 最多 {} 个",
                reference.key,
                max
            );
        }
        if count > 1 {
            ensure!(
                reference.multiple,
                "参考输入 {} 只允许一个文件",
                reference.key
            );
        }
        total += count;
    }
    ensure!(
        total <= limits(model).references,
        "此模型最多 {} 个参考输入",
        limits(model).references
    );
    Ok(())
}

pub fn validate(model: &MediaModel) -> Result<()> {
    let Some(capabilities) = &model.capabilities else {
        return Ok(());
    };
    if let Some(references) = &capabilities.references {
        ensure!(
            references.len() <= MAX_REFERENCE_ENTRIES,
            "参考输入最多声明 {MAX_REFERENCE_ENTRIES} 条"
        );
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
        capabilities
            .reference_limit
            .is_none_or(|limit| (1..=MAX_REFERENCES as u32).contains(&limit)),
        "参考输入总数上限不能超过系统上限 {MAX_REFERENCES}"
    );
    ensure!(
        capabilities
            .reference_seconds
            .is_none_or(|seconds| seconds > 0. && seconds <= MAX_REFERENCE_SECONDS),
        "参考视频总时长上限不能超过系统上限 {MAX_REFERENCE_SECONDS} 秒"
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
    Ok(())
}

/// Frozen snapshot of the capabilities the pre-declaration code hardcoded per endpoint.
///
/// Migration only. New models declare their own capabilities, and this table is deleted
/// once no stored model predates the schema.
pub fn legacy_declaration(plugin: &str, endpoint: &str) -> Option<Capabilities> {
    if plugin != "fal" {
        return None;
    }
    let image = |key: &str, required: bool| ReferenceDeclaration {
        key: key.into(),
        kind: "image".into(),
        role: "reference".into(),
        multiple: true,
        required,
        max: None,
    };
    let multi = |key: &str, kind: &str, max: u32| ReferenceDeclaration {
        key: key.into(),
        kind: kind.into(),
        role: "reference".into(),
        multiple: true,
        required: false,
        max: Some(max),
    };
    let frame = |key: &str, role: &str| ReferenceDeclaration {
        key: key.into(),
        kind: "image".into(),
        role: role.into(),
        multiple: false,
        required: false,
        max: None,
    };
    let control = |path: &str, values: Option<&[&str]>, min: Option<u32>, max: Option<u32>| {
        ControlDeclaration {
            path: path.into(),
            values: values.map(|values| values.iter().map(|v| (*v).to_string()).collect()),
            min,
            max,
        }
    };
    let video_ratios = ["9:16", "16:9", "1:1", "3:4", "4:3", "21:9"];
    let long_video_resolutions = ["480P", "768P", "2K", "4K"];
    let short_video_resolutions = ["480P", "768P", "1080P"];
    let image_ratios = ["1:1", "9:16", "16:9", "3:4", "4:3", "2:3", "3:2"];
    let image_resolutions = ["1K", "2K", "4K"];
    let duration = || control("/duration", None, Some(5), Some(15));
    match endpoint {
        "minimax/h3/text-to-video" => Some(Capabilities {
            controls: Some(Controls {
                aspect_ratio: Some(control("/aspect_ratio", Some(&video_ratios), None, None)),
                resolution: Some(control(
                    "/resolution",
                    Some(&long_video_resolutions),
                    None,
                    None,
                )),
                duration: Some(duration()),
                image_size: None,
            }),
            ..Default::default()
        }),
        "minimax/h3/image-to-video" => Some(Capabilities {
            references: Some(vec![
                frame("/image_url", "first-frame"),
                frame("/end_image_url", "last-frame"),
                ReferenceDeclaration {
                    key: "/target_audio_url".into(),
                    kind: "audio".into(),
                    role: "reference".into(),
                    multiple: false,
                    required: false,
                    max: None,
                },
            ]),
            controls: Some(Controls {
                aspect_ratio: None,
                resolution: Some(control(
                    "/resolution",
                    Some(&long_video_resolutions),
                    None,
                    None,
                )),
                duration: Some(duration()),
                image_size: None,
            }),
            ..Default::default()
        }),
        "minimax/h3/reference-to-video" => Some(Capabilities {
            references: Some(vec![
                multi("/reference_image_urls", "image", 9),
                multi("/reference_video_urls", "video", 3),
                multi("/reference_audio_urls", "audio", 3),
            ]),
            reference_limit: Some(12),
            reference_seconds: Some(15.),
            controls: Some(Controls {
                aspect_ratio: Some(control("/aspect_ratio", Some(&video_ratios), None, None)),
                resolution: Some(control(
                    "/resolution",
                    Some(&long_video_resolutions),
                    None,
                    None,
                )),
                duration: Some(duration()),
                image_size: None,
            }),
        }),
        "minimax/h3-max/image-to-video" => Some(Capabilities {
            references: Some(vec![frame("/image_url", "first-frame")]),
            controls: Some(Controls {
                aspect_ratio: None,
                resolution: Some(control(
                    "/resolution",
                    Some(&short_video_resolutions),
                    None,
                    None,
                )),
                duration: Some(duration()),
                image_size: None,
            }),
            ..Default::default()
        }),
        "minimax/h3-max/text-to-video" | "minimax/h3-max/reference-to-video" => {
            Some(Capabilities {
                controls: Some(Controls {
                    aspect_ratio: Some(control("/aspect_ratio", Some(&video_ratios), None, None)),
                    resolution: Some(control(
                        "/resolution",
                        Some(&short_video_resolutions),
                        None,
                        None,
                    )),
                    duration: Some(duration()),
                    image_size: None,
                }),
                ..Default::default()
            })
        }
        "openai/gpt-image-2.5/sunburst/text-to-image"
        | "openai/gpt-image-2.5/flare/text-to-image" => Some(Capabilities {
            controls: Some(Controls {
                image_size: Some(control("/image_size", None, None, None)),
                ..Default::default()
            }),
            ..Default::default()
        }),
        "openai/gpt-image-2.5/sunburst/edit" | "openai/gpt-image-2.5/flare/edit" => {
            Some(Capabilities {
                references: Some(vec![image("/image_urls", true)]),
                controls: Some(Controls {
                    image_size: Some(control("/image_size", None, None, None)),
                    ..Default::default()
                }),
                ..Default::default()
            })
        }
        "fal-ai/nano-banana-2" => Some(Capabilities {
            controls: Some(Controls {
                aspect_ratio: Some(control("/aspect_ratio", Some(&image_ratios), None, None)),
                resolution: Some(control("/resolution", Some(&image_resolutions), None, None)),
                ..Default::default()
            }),
            ..Default::default()
        }),
        "fal-ai/nano-banana-2/edit" => Some(Capabilities {
            references: Some(vec![image("/image_urls", true)]),
            controls: Some(Controls {
                aspect_ratio: Some(control("/aspect_ratio", Some(&image_ratios), None, None)),
                resolution: Some(control("/resolution", Some(&image_resolutions), None, None)),
                ..Default::default()
            }),
            ..Default::default()
        }),
        _ => None,
    }
}

/// Stamps declarations onto stored models once, then never runs again.
pub fn migrate(db: &rusqlite::Connection) -> Result<()> {
    if super::setting(db, MIGRATION_KEY)?.is_some() {
        return Ok(());
    }
    let mut models = super::media::read(db)?;
    let mut changed = false;
    for model in &mut models {
        if model.capabilities.is_some() {
            continue;
        }
        if let Some(declared) = legacy_declaration(&model.plugin, &model.endpoint) {
            model.capabilities = Some(declared);
            changed = true;
        }
    }
    if changed {
        super::media::write(db, &models)?;
    }
    db.execute(
        "INSERT INTO settings VALUES(?1,'1') ON CONFLICT(key) DO UPDATE SET value='1'",
        [MIGRATION_KEY],
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "capabilities_tests.rs"]
mod tests;
