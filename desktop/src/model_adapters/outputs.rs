use super::ModelOutput;
use anyhow::{Context, Result, ensure};
use serde_json::Value;

// Report the failing field, never the response value (which may contain media
// data, private URLs or an echoed request).
pub(super) fn normalized_outputs(result: &Value, provider: &str) -> Result<Vec<ModelOutput>> {
    let values = result["outputs"]
        .as_array()
        .with_context(|| format!("{provider} 结果格式无效：outputs 须为数组"))?;
    let mut outputs = Vec::with_capacity(values.len());
    for (index, value) in values.iter().enumerate() {
        let kind = value["kind"]
            .as_str()
            .filter(|kind| matches!(*kind, "image" | "video" | "audio"))
            .with_context(|| {
                format!("{provider} 结果格式无效：outputs[{index}].kind 须为媒体类型")
            })?;
        let url = value["url"]
            .as_str()
            .filter(|url| !url.trim().is_empty())
            .with_context(|| {
                format!("{provider} 结果格式无效：outputs[{index}].url 须为非空字符串")
            })?;
        outputs.push(ModelOutput {
            kind: kind.into(),
            url: url.into(),
        });
    }
    ensure!(
        !outputs.is_empty(),
        "{provider} 未返回媒体结果：outputs 为空"
    );
    Ok(outputs)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn malformed_outputs_identify_the_field_without_echoing_values() {
        let error = normalized_outputs(
            &json!({"outputs":[{"kind":"image","url":{"token":"secret"}}]}),
            "任务",
        )
        .unwrap_err();
        assert!(error.to_string().contains("outputs[0].url"));
        assert!(!error.to_string().contains("secret"));
        assert!(normalized_outputs(&json!({}), "任务").is_err());
        assert!(normalized_outputs(&json!({"outputs":[]}), "任务").is_err());
    }
    #[test]
    fn a_malformed_output_is_never_silently_dropped_from_the_batch() {
        let errors = [
            (
                super::super::fal::normalize_outputs(
                    &json!({"images":[{"url":"https://example.com/a.png"},{}]}),
                )
                .unwrap_err(),
                "images[1].url",
            ),
            (
                super::super::codex::CODEX
                    .outputs(&json!({"data":[{"b64_json":"YQ=="},{}]}))
                    .unwrap_err(),
                "data[1].b64_json",
            ),
            (
                super::super::dashscope::outputs(&json!({"output":{"video_url":42}})).unwrap_err(),
                "output.video_url",
            ),
        ];
        for (error, field) in errors {
            assert!(error.to_string().contains(field));
        }
    }
}
