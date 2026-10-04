//! Unsupported schema keywords are registration errors, never silently ignored.
use serde_json::Value;
pub fn check(schema: &Value) -> Result<(), String> {
    let object = schema.as_object().ok_or("Schema must be an object")?;
    for (key, value) in object {
        match key.as_str() {
            "type" => {
                let valid = |v: &Value| {
                    matches!(
                        v.as_str(),
                        Some(
                            "object"
                                | "array"
                                | "string"
                                | "boolean"
                                | "null"
                                | "integer"
                                | "number"
                        )
                    )
                };
                if !valid(value)
                    && !value
                        .as_array()
                        .is_some_and(|items| !items.is_empty() && items.iter().all(valid))
                {
                    return Err("Unsupported schema type".into());
                }
            }
            "properties" => {
                for child in value
                    .as_object()
                    .ok_or("Invalid schema properties")?
                    .values()
                {
                    check(child)?;
                }
            }
            "items" => check(value)?,
            "oneOf" => {
                for child in value.as_array().ok_or("Invalid schema union")? {
                    check(child)?;
                }
            }
            "additionalProperties" if !value.is_boolean() => {
                return Err("additionalProperties must be boolean".into());
            }
            "required" | "enum" if !value.is_array() => {
                return Err(format!("{key} must be an array"));
            }
            "minimum" | "maximum" | "exclusiveMinimum" | "minLength" | "maxLength" | "minItems"
            | "maxItems"
                if !value.is_number() =>
            {
                return Err(format!("{key} must be numeric"));
            }
            "const"
            | "enum"
            | "required"
            | "additionalProperties"
            | "minimum"
            | "maximum"
            | "exclusiveMinimum"
            | "minLength"
            | "maxLength"
            | "minItems"
            | "maxItems"
            | "description"
            | "title"
            | "default" => {}
            _ => return Err(format!("Unsupported schema keyword: {key}")),
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn contracts_are_supported_and_nested_unsupported_constraints_are_rejected() {
        check(&crate::assistant::tool_schema::schema()).unwrap();
        assert!(
            check(&json!({"type":"object","properties":{"value":{"type":"string","pattern":"x"}}}))
                .is_err()
        );
        let schema = json!({"type":"object","properties":{"visual":{"type":["object","null"],"properties":{"curve":{"type":"array","minItems":2,"items":{"type":"number"}}},"additionalProperties":false}},"additionalProperties":false});
        assert!(super::super::schema::validate(&schema, &json!({"visual":null})).is_ok());
        let errors = super::super::schema::issues(
            &schema,
            &json!({"visual":{"curve":[0.2],"unknown":1},"extra":2}),
        );
        assert_eq!(errors.len(), 3);
    }
}
