//! Validate our declared JSON-schema subset, collecting path-qualified issues.
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
#[derive(Debug, Serialize, Deserialize)]
pub struct Issue {
    pub path: String,
    pub message: String,
}
#[cfg(test)]
pub fn validate(schema: &Value, value: &Value) -> Result<(), String> {
    let issues = issues(schema, value);
    if issues.is_empty() {
        Ok(())
    } else {
        Err(serde_json::to_string(&issues).unwrap())
    }
}
pub fn issues(schema: &Value, value: &Value) -> Vec<Issue> {
    if let Err(message) = super::schema_definition::check(schema) {
        return vec![Issue {
            path: "$schema".into(),
            message,
        }];
    }
    let mut output = vec![];
    visit(schema, value, "$", &mut output);
    output
}
pub fn rejection(issues: Vec<Issue>) -> Value {
    json!({"error":"Invalid tool arguments", "code":"INVALID_ARGS", "stage":"validation", "outcome":"not_executed", "issues":issues,
        "recovery":{"action":"correct_arguments","message":"Correct all listed fields and retry. No project reads are needed for argument errors."}})
}
fn visit(s: &Value, v: &Value, path: &str, out: &mut Vec<Issue>) {
    let mut issue = |message: String| {
        out.push(Issue {
            path: path.into(),
            message,
        })
    };
    if let Some(branches) = s["oneOf"].as_array() {
        let results: Vec<_> = branches
            .iter()
            .map(|b| {
                let mut errors = vec![];
                visit(b, v, path, &mut errors);
                (b, errors)
            })
            .collect();
        let matches = results
            .iter()
            .filter(|(_, errors)| errors.is_empty())
            .count();
        if matches == 1 {
            return;
        }
        if matches > 1 {
            issue("Ambiguous union argument".into());
            return;
        }
        // Exact discriminators select the diagnostic branch, never normalize
        // input. Other discriminators can locate an image operation even when
        // `op` is missing, so all actionable errors return in the same round.
        let selected = results.iter().filter_map(|(b, errors)| {
            let properties = b["properties"].as_object()?;
            let mut matches = 0;
            let mut conflicts = 0;
            for (key, schema) in properties {
                if let (Some(expected), Some(actual)) = (schema.get("const"), v.get(key)) {
                    if expected == actual {
                        matches += 1;
                    } else {
                        conflicts += 1;
                    }
                }
            }
            (matches > 0).then_some((conflicts, std::cmp::Reverse(matches), errors))
        });
        if let Some((_, _, errors)) =
            selected.min_by_key(|(conflicts, matches, errors)| (*conflicts, *matches, errors.len()))
        {
            out.extend(errors.iter().map(|e| Issue {
                path: e.path.clone(),
                message: e.message.clone(),
            }));
        } else {
            let allowed: Vec<_> = branches
                .iter()
                .filter_map(|b| b["properties"]["op"]["const"].as_str())
                .collect::<std::collections::BTreeSet<_>>()
                .into_iter()
                .collect();
            if !allowed.is_empty() {
                out.push(Issue {
                    path: format!("{path}.op"),
                    message: format!(
                        "{}; allowed operations: {}",
                        if v.get("op").is_none() {
                            "Required field missing"
                        } else {
                            "Unknown operation"
                        },
                        json!(allowed)
                    ),
                });
            } else {
                issue("Argument does not match an available variant".into());
            }
        }
        return;
    }
    let matches_type = |kind: &str| match kind {
        "object" => v.is_object(),
        "array" => v.is_array(),
        "string" => v.is_string(),
        "boolean" => v.is_boolean(),
        "null" => v.is_null(),
        "integer" => v.as_f64().is_some_and(|n| n.fract() == 0.),
        "number" => v.is_number(),
        _ => false,
    };
    let valid = match &s["type"] {
        Value::String(kind) => matches_type(kind),
        Value::Array(kinds) => kinds.iter().filter_map(Value::as_str).any(matches_type),
        Value::Null => true,
        _ => false,
    };
    if !valid {
        issue(format!("Expected {}", s["type"]));
        return;
    }
    if let Some(expected) = s.get("const")
        && expected != v
    {
        issue(format!("Expected {expected}"));
    }
    if let Some(values) = s["enum"].as_array()
        && !values.contains(v)
    {
        let guidance = s["description"].as_str().unwrap_or("");
        issue(format!("Allowed values: {}. {guidance}", s["enum"]));
    }
    if let Some(object) = v.as_object() {
        for key in s["required"]
            .as_array()
            .into_iter()
            .flatten()
            .filter_map(Value::as_str)
        {
            if !object.contains_key(key) {
                out.push(Issue {
                    path: format!("{path}.{key}"),
                    message: s["properties"][key].get("const").map_or_else(
                        || "Required field missing".into(),
                        |value| format!("Required field missing; expected {value}"),
                    ),
                });
            }
        }
        for (key, item) in object {
            let child_path = format!("{path}.{key}");
            if let Some(child) = s["properties"].get(key) {
                visit(child, item, &child_path, out);
            } else if s["additionalProperties"] == false {
                out.push(Issue {
                    path: child_path,
                    message: format!(
                        "Field is not accepted by this operation; allowed fields: {}",
                        json!(
                            s["properties"]
                                .as_object()
                                .map(|p| p.keys().collect::<Vec<_>>())
                                .unwrap_or_default()
                        )
                    ),
                });
            }
        }
    }
    if let Some(items) = v.as_array() {
        for (key, invalid) in [
            (
                "minItems",
                s["minItems"]
                    .as_u64()
                    .is_some_and(|n| items.len() < n as usize),
            ),
            (
                "maxItems",
                s["maxItems"]
                    .as_u64()
                    .is_some_and(|n| items.len() > n as usize),
            ),
        ] {
            if invalid {
                out.push(Issue {
                    path: path.into(),
                    message: format!("{key}: {}", s[key]),
                });
            }
        }
        if let Some(child) = s.get("items") {
            for (index, item) in items.iter().enumerate() {
                visit(child, item, &format!("{path}[{index}]"), out);
            }
        }
    }
    if let Some(text) = v.as_str() {
        let n = text.chars().count() as u64;
        for (key, invalid) in [
            (
                "minLength",
                s["minLength"].as_u64().is_some_and(|min| n < min),
            ),
            (
                "maxLength",
                s["maxLength"].as_u64().is_some_and(|max| n > max),
            ),
        ] {
            if invalid {
                out.push(Issue {
                    path: path.into(),
                    message: format!("{key}: {}", s[key]),
                });
            }
        }
    }
    if let Some(n) = v.as_f64() {
        for (key, invalid) in [
            ("minimum", s["minimum"].as_f64().is_some_and(|v| n < v)),
            (
                "exclusiveMinimum",
                s["exclusiveMinimum"].as_f64().is_some_and(|v| n <= v),
            ),
            ("maximum", s["maximum"].as_f64().is_some_and(|v| n > v)),
        ] {
            if invalid {
                out.push(Issue {
                    path: path.into(),
                    message: format!("{key}: {}", s[key]),
                });
            }
        }
    }
}
