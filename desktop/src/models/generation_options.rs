//! Per-generation controls shared with the frontend; never connection settings.
use super::media::MediaModel;
use anyhow::{Context, Result, ensure};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

#[derive(Clone, Serialize, Deserialize)]
pub struct GenerationOption {
    pub key: String,
    pub label: String,
    pub path: String,
    #[serde(rename = "type")]
    pub value_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub values: Option<Vec<Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub min: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<f64>,
}

pub fn declarations(model: &MediaModel) -> Result<Vec<GenerationOption>> {
    let rules: Value = serde_json::from_str(include_str!(
        "../../../frontend/src/models/config/generation-options.json"
    ))?;
    let options = rules
        .as_array()
        .unwrap()
        .iter()
        .find(|rule| {
            rule["plugin"] == model.plugin
                && (rule.get("model").is_none() || rule["model"] == model.params["model"])
                && rule["endpoints"]
                    .as_array()
                    .is_none_or(|endpoints| endpoints.iter().any(|e| e == &model.endpoint))
        })
        .map(|rule| rule["options"].clone())
        .unwrap_or(json!([]));
    Ok(serde_json::from_value(options)?)
}

pub fn normalized(option: &GenerationOption, value: &Value) -> Value {
    match (option.value_type.as_str(), value.as_str()) {
        ("boolean", Some("true")) => json!(true),
        ("boolean", Some("false")) => json!(false),
        ("integer", Some(v)) => v.parse::<i64>().map_or(value.clone(), |v| json!(v)),
        ("number", Some(v)) => v.parse::<f64>().map_or(value.clone(), |v| json!(v)),
        _ => value.clone(),
    }
}

pub fn validate(option: &GenerationOption, value: &Value) -> Result<()> {
    let valid = match option.value_type.as_str() {
        "string" => value.is_string(),
        "boolean" => value.is_boolean(),
        "number" => value.as_f64().is_some_and(f64::is_finite),
        "integer" => value
            .as_f64()
            .is_some_and(|v| v.is_finite() && v.fract() == 0.),
        _ => false,
    };
    ensure!(
        valid
            && option
                .values
                .as_ref()
                .is_none_or(|values| values.contains(value))
            && value
                .as_f64()
                .is_none_or(|v| option.min.is_none_or(|min| v >= min)
                    && option.max.is_none_or(|max| v <= max)),
        "生成参数「{}」的取值不受支持",
        option.label
    );
    Ok(())
}

pub fn validate_options<'a>(
    options: &'a Value,
    declared: &'a [GenerationOption],
) -> Result<Vec<(&'a GenerationOption, &'a Value)>> {
    let Some(options) = options.as_object().filter(|_| !options.is_null()) else {
        ensure!(options.is_null(), "生成选项必须是对象");
        return Ok(Vec::new());
    };
    options
        .iter()
        .map(|(key, value)| {
            let option = declared
                .iter()
                .find(|o| o.key == *key)
                .context("此模型不支持指定的生成选项")?;
            validate(option, value)?;
            Ok((option, value))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn declared_options_reject_wrong_types_unknown_fields_and_out_of_range_counts() {
        let model: MediaModel = serde_json::from_value(json!({"id":"image","name":"Image","kind":"image","plugin":"fal","endpoint":"openai/gpt-image-2.5/sunburst/text-to-image","params":{},"enabled":true})).unwrap();
        let options = declarations(&model).unwrap();
        assert_eq!(options[0].value_type, "integer");
        assert!(validate_options(&json!({"num_images":2,"quality":"high"}), &options).is_ok());
        for invalid in [
            json!({"num_images":20}),
            json!({"num_images":"2"}),
            json!({"quality":false}),
            json!({"unknown":300}),
            json!({"num_images":{"nested":2}}),
        ] {
            assert!(validate_options(&invalid, &options).is_err());
        }
        let video: MediaModel = serde_json::from_value(json!({"id":"video","name":"Video","kind":"video","plugin":"dashscope","endpoint":"https://example.com/video-synthesis","params":{"model":"wan3.0-video"},"enabled":true})).unwrap();
        let options = declarations(&video).unwrap();
        assert_eq!(options[0].value_type, "boolean");
        assert!(validate_options(&json!({"audio":false}), &options).is_ok());
        assert!(validate_options(&json!({"audio":"false"}), &options).is_err());
    }
}
