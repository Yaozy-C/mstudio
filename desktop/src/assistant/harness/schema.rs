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
        return Err("Argument type does not match the tool schema".into());
    }
    if let Some(values) = schema["enum"].as_array()
        && !values.contains(value)
    {
        return Err(format!(
            "Argument not in enum; allowed values: {}",
            schema["enum"]
        ));
    }
    if let Some(object) = value.as_object() {
        if let Some(required) = schema["required"].as_array() {
            for field in required.iter().filter_map(Value::as_str) {
                if !object.contains_key(field) {
                    return Err(format!("Missing required argument: {field}"));
                }
            }
        }
        for (key, item) in object {
            if let Some(child) = schema["properties"].get(key) {
                validate(child, item).map_err(|e| format!("{key}: {e}"))?;
            } else if schema["additionalProperties"] == false {
                return Err(format!("Unknown argument: {key}"));
            }
        }
    }
    if let Some(items) = value.as_array() {
        if schema["maxItems"]
            .as_u64()
            .is_some_and(|n| items.len() > n as usize)
        {
            return Err("Too many array items".into());
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
        return Err("Argument text too long".into());
    }
    if let Some(n) = value.as_f64()
        && (schema["minimum"].as_f64().is_some_and(|v| n < v)
            || schema["exclusiveMinimum"].as_f64().is_some_and(|v| n <= v)
            || schema["maximum"].as_f64().is_some_and(|v| n > v))
    {
        return Err("Number outside allowed range".into());
    }
    Ok(())
}
