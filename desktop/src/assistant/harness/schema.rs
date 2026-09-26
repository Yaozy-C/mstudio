//! Validate the schema subset authored by Mstudio before entering any tool body.
use serde_json::Value;
pub fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    let valid = match schema["type"].as_str() {
        Some("object") => value.is_object(),
        Some("array") => value.is_array(),
        Some("string") => value.is_string(),
        Some("boolean") => value.is_boolean(),
        Some("integer") => value.is_i64() || value.is_u64(),
        Some("number") => value.is_number(),
        None => true,
        _ => false,
    };
    if !valid {
        return Err("参数类型与工具定义不符".into());
    }
    if let Some(values) = schema["enum"].as_array()
        && !values.contains(value)
    {
        return Err(format!("参数不在允许值内；允许值：{}", schema["enum"]));
    }
    if let Some(object) = value.as_object() {
        if let Some(required) = schema["required"].as_array() {
            for field in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(field) {
                    return Err(format!("缺少必填参数：{field}"));
                }
            }
        }
        for (key, item) in object {
            if let Some(child) = schema["properties"].get(key) {
                validate(child, item).map_err(|e| format!("{key}: {e}"))?;
            } else if schema["additionalProperties"] == false {
                return Err(format!("未知参数：{key}"));
            }
        }
    }
    if let Some(items) = value.as_array() {
        if schema["maxItems"]
            .as_u64()
            .is_some_and(|n| items.len() > n as usize)
        {
            return Err("数组项目过多".into());
        }
        if let Some(child) = schema.get("items") {
            for (index, item) in items.iter().enumerate() {
                validate(child, item).map_err(|e| format!("[{index}]: {e}"))?;
            }
        }
    }
    if let Some(text) = value.as_str()
        && schema["maxLength"]
            .as_u64()
            .is_some_and(|n| text.chars().count() > n as usize)
    {
        return Err("参数文字过长".into());
    }
    if let Some(n) = value.as_f64()
        && (schema["minimum"].as_f64().is_some_and(|v| n < v)
            || schema["exclusiveMinimum"].as_f64().is_some_and(|v| n <= v)
            || schema["maximum"].as_f64().is_some_and(|v| n > v))
    {
        return Err("数值超出允许范围".into());
    }
    Ok(())
}
