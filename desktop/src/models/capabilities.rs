//! Model-declared reference inputs and generation controls.
//!
//! A protocol adapter defines how bytes travel; what a model accepts is a model fact, so it
//! is stored with the model and validated here. The declaration is data, never code: field
//! paths are JSON Pointers checked against the request body vocabulary, roles and kinds
//! come from fixed lists, and limits apply only when explicitly configured.
use super::media::MediaModel;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};

const KINDS: [&str; 3] = ["image", "video", "audio"];
const ROLES: [&str; 4] = ["reference", "first-frame", "last-frame", "mask"];
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
    pub min: Option<f64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
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
    pub ratio_from_reference: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub references: Option<Vec<ReferenceDeclaration>>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub controls: Option<Controls>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_limit: Option<u32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reference_seconds: Option<f64>,
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
    let defaults: std::collections::HashMap<String, Capabilities> = serde_json::from_str(
        include_str!("../../../frontend/src/models/config/protocols.json"),
    )
    .expect("valid protocol configuration");
    defaults
        .get(&model.plugin)
        .and_then(|c| c.references.clone())
        .unwrap_or_default()
}

/// Optional total limits; role-level limits are enforced per field.
pub struct Limits {
    pub references: Option<usize>,
    pub seconds: Option<f64>,
}

pub fn limits(model: &MediaModel) -> Limits {
    let declared = model.capabilities.as_ref();
    Limits {
        references: declared
            .and_then(|capabilities| capabilities.reference_limit)
            .map(|limit| limit as usize),
        seconds: declared.and_then(|capabilities| capabilities.reference_seconds),
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
            Some(serde_json::Value::Array(items)) => {
                ensure!(
                    reference.multiple
                        && items
                            .iter()
                            .all(|v| v.as_str().is_some_and(|s| !s.is_empty())),
                    "参考输入 {} 必须为有效的 URL 数组",
                    reference.key
                );
                items.len()
            }
            Some(value) => {
                ensure!(
                    !reference.multiple && value.as_str().is_some_and(|s| !s.is_empty()),
                    "参考输入 {} 的格式与声明不一致",
                    reference.key
                );
                1
            }
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
    if let Some(limit) = limits(model).references {
        ensure!(total <= limit, "此模型最多 {limit} 个参考输入");
    }
    Ok(())
}

#[path = "capabilities_validation.rs"]
mod validation;
pub use validation::validate;

#[path = "capabilities_legacy.rs"]
mod legacy;
#[cfg(test)]
pub use legacy::legacy_declaration;
pub use legacy::migrate;

#[cfg(test)]
#[path = "capabilities_tests.rs"]
mod tests;
