//! Native project rules. Only JSON data contracts are shared with the UI.
mod creative;
mod edit;
mod generation;
mod inspect;
mod layout;
mod merge;
mod receipt;
mod timeline;
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};
use std::collections::HashSet;

pub(super) fn rows(value: &Value) -> &[Value] {
    value.as_array().map(Vec::as_slice).unwrap_or(&[])
}
pub(super) fn text(value: &Value) -> &str {
    value.as_str().unwrap_or("")
}
pub(super) fn num(value: &Value, default: f64) -> f64 {
    value.as_f64().unwrap_or(default)
}
#[cfg(test)]
thread_local! {static TEST_SEQUENCE: std::cell::Cell<Option<usize>> = const {std::cell::Cell::new(None)};}
pub(super) fn id() -> String {
    #[cfg(test)]
    if let Some(seq) = TEST_SEQUENCE.with(|v| v.get()) {
        TEST_SEQUENCE.with(|v| v.set(Some(seq + 1)));
        return format!("native-generated-{}", seq + 1);
    }
    mstudio::media::id()
}
pub(super) fn now() -> Result<u64> {
    #[cfg(test)]
    if TEST_SEQUENCE.with(|v| v.get().is_some()) {
        return Ok(1793000000000);
    }
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64)
}
fn normalize_numbers(v: &mut Value) {
    match v {
        Value::Number(n) if n.is_f64() => {
            let f = n.as_f64().unwrap();
            if f.fract() == 0. && f.abs() < 9_007_199_254_740_992. {
                *n = serde_json::Number::from(f as i64);
            }
        }
        Value::Array(a) => a.iter_mut().for_each(normalize_numbers),
        Value::Object(m) => m.values_mut().for_each(normalize_numbers),
        _ => {}
    }
}
pub(super) fn stringify(value: &Value) -> String {
    let mut v = value.clone();
    normalize_numbers(&mut v);
    v.to_string()
}
pub(super) fn obj(value: &mut Value) -> &mut serde_json::Map<String, Value> {
    value.as_object_mut().expect("validated project object")
}
pub(super) fn array(value: &mut Value) -> &mut Vec<Value> {
    if !value.is_array() {
        *value = json!([]);
    }
    value.as_array_mut().unwrap()
}
pub(super) fn assign(target: &mut Value, patch: &Value) {
    if let Some(fields) = patch.as_object() {
        obj(target).extend(fields.clone());
    }
}
pub(super) fn select(value: &Value, keys: &[&str]) -> Value {
    let mut out = json!({});
    for key in keys {
        if let Some(v) = value.get(*key) {
            out[*key] = v.clone();
        }
    }
    out
}
pub(super) fn checked_text(value: &Value, max: usize) -> Result<&str> {
    let s = value.as_str().context("文字字段无效或过长")?;
    ensure!(s.encode_utf16().count() <= max, "文字字段无效或过长");
    Ok(s)
}
pub(super) fn checked_number(value: &Value) -> Result<f64> {
    value.as_f64().context("数值字段无效")
}
pub(super) fn node<'a>(p: &'a Value, id: &Value) -> Option<&'a Value> {
    rows(&p["nodes"]).iter().find(|n| &n["id"] == id)
}
pub(super) fn asset<'a>(p: &'a Value, id: &Value) -> Option<&'a Value> {
    rows(&p["assets"])
        .iter()
        .chain(rows(&p["previsAssets"]))
        .find(|a| &a["id"] == id)
}
pub(super) fn snippet(value: &Value, offset: usize, limit: usize) -> Value {
    let s = text(value);
    let units: Vec<_> = s.encode_utf16().collect();
    let start = offset.min(units.len());
    let end = (start + limit).min(units.len());
    json!({"text":String::from_utf16_lossy(&units[start..end]),"nextTextOffset":if end<units.len(){json!(end)}else{Value::Null}})
}
pub(super) fn clipped(value: &Value, limit: usize) -> String {
    text(&snippet(value, 0, limit)["text"]).to_owned()
}
pub(super) fn offset(value: &Value) -> usize {
    num(value, 0.).max(0.).floor() as usize
}
pub(super) fn title(task: &Value) -> &str {
    task["title"]
        .as_str()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .unwrap_or("未命名")
}
pub(super) fn equal(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Number(a), Value::Number(b)) => {
            a == b || (a.as_f64() == b.as_f64() && (a.is_f64() || b.is_f64()))
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len() && a.iter().zip(b).all(|(a, b)| equal(a, b))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len() && a.iter().all(|(k, v)| b.get(k).is_some_and(|w| equal(v, w)))
        }
        _ => a == b,
    }
}
fn except(a: &Value, b: &Value, keys: &[&str]) -> bool {
    let (Some(a), Some(b)) = (a.as_object(), b.as_object()) else {
        return equal(a, b);
    };
    a.keys()
        .chain(b.keys())
        .filter(|key| !keys.contains(&key.as_str()))
        .all(|key| match (a.get(key), b.get(key)) {
            (Some(a), Some(b)) => equal(a, b),
            (None, None) => true,
            _ => false,
        })
}
pub(super) fn content_changed(a: &Value, b: &Value) -> bool {
    if !except(
        a,
        b,
        &[
            "revision",
            "storageVersion",
            "previsAssets",
            "updated",
            "viewport",
            "nodes",
            "production",
        ],
    ) {
        return true;
    }
    if !except(
        &a.get("production").cloned().unwrap_or(json!({})),
        &b.get("production").cloned().unwrap_or(json!({})),
        &["positions", "viewport", "drafts"],
    ) {
        return true;
    }
    let at = a["production"]["drafts"].as_object();
    let bt = b["production"]["drafts"].as_object();
    let keys: HashSet<_> = at
        .into_iter()
        .flat_map(|m| m.keys())
        .chain(bt.into_iter().flat_map(|m| m.keys()))
        .collect();
    for k in keys {
        let x = at.and_then(|m| m.get(k));
        let y = bt.and_then(|m| m.get(k));
        if x.is_none()
            || y.is_none()
            || !except(
                x.unwrap(),
                y.unwrap(),
                &[
                    "status",
                    "progress",
                    "error",
                    "trackingPaused",
                    "jobId",
                    "submissionId",
                    "requestId",
                ],
            )
        {
            return true;
        }
    }
    rows(&a["nodes"]).len() != rows(&b["nodes"]).len()
        || rows(&b["nodes"]).iter().any(|n| {
            node(a, &n["id"]).is_none_or(|old| !except(old, n, &["x", "y", "width", "height"]))
        })
}
#[derive(Debug)]
pub(crate) struct ExecutionError(pub String);
impl std::fmt::Display for ExecutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}
impl std::error::Error for ExecutionError {}
#[derive(Debug)]
struct Rejection {
    code: &'static str,
    message: String,
    context: Value,
}
impl std::fmt::Display for Rejection {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.message)
    }
}
impl std::error::Error for Rejection {}
fn reject(code: &'static str, message: String, context: Value) -> anyhow::Error {
    Rejection {
        code,
        message,
        context,
    }
    .into()
}
fn translate(message: &str) -> String {
    static MESSAGES: std::sync::LazyLock<Value> = std::sync::LazyLock::new(|| {
        serde_json::from_str(include_str!(
            "../../../../frontend/src/assistant/toolMessages.json"
        ))
        .expect("valid tool diagnostics")
    });
    if let Some(s) = MESSAGES[message].as_str() {
        return s.to_owned();
    }
    for (prefix, suffix, before, after) in [
        (
            "不支持的调色参数：",
            "",
            "Unsupported grade parameter: ",
            "",
        ),
        (
            "不支持的脚本字段：",
            "",
            "Unsupported screenplay field: ",
            "",
        ),
        ("不支持的镜头字段：", "", "Unsupported shot field: ", ""),
        (
            "调色参数 ",
            " 超出范围",
            "Visual parameter ",
            " outside allowed range",
        ),
        ("", " 超出允许范围", "", " outside allowed range"),
        ("", " 控制点无效", "Invalid ", " control points"),
        ("", "必须为布尔值", "", " must be a boolean"),
        (
            "",
            "必须是不重复的段落 ID 数组",
            "",
            " must be an array of unique paragraph IDs",
        ),
        (
            "「",
            "」素材与计划时长不匹配，请先调整镜头时长",
            "Media for '",
            "' does not match planned duration; adjust shot timing first",
        ),
        (
            "请使用用户选择的模型 ",
            "；省略 mediaModelId 可沿用本轮选择。",
            "Use the user-selected model ",
            "; omit mediaModelId to retain the selection.",
        ),
    ] {
        if let Some(value) = message
            .strip_prefix(prefix)
            .and_then(|s| s.strip_suffix(suffix))
        {
            return format!("{before}{value}{after}");
        }
    }
    message.to_owned()
}
fn model_error(raw: &str) -> String {
    if let Ok(issue) = serde_json::from_str::<Value>(raw)
        && issue["code"].is_string()
        && issue["message"].is_string()
    {
        let mut out = json!({"code":issue["code"],"message":translate(issue["details"].as_str().unwrap_or(text(&issue["message"])))});
        for k in ["retryable", "httpStatus", "stage", "outcome"] {
            if let Some(v) = issue.get(k) {
                out[k] = v.clone();
            }
        }
        return out.to_string();
    }
    translate(raw)
}
pub fn execute(input: Value) -> Result<Value> {
    let action = text(&input["action"]);
    if action != "capabilities" && !input["document"].is_object() {
        return Err(ExecutionError(format!(
            "Project execution failed ({action}): missing project document"
        ))
        .into());
    }
    let result = match action {
        "merge" => merge::project(&input),
        "prune_deleted" => Ok(
            json!({"document":merge::prune(&input["document"],rows(&input["current"]["removedAssetIds"]))}),
        ),
        "inspect" => inspect::inspect(&input["document"], &input["args"]),
        "edit" => edit::apply(&input),
        "result_name" => generation::result_name(&input["document"], &input["job"])
            .map(|name| json!({"name":name})),
        "job_tracking" => {
            Ok(json!({"document":generation::tracking(input["document"].clone(),&input["job"])}))
        }
        "recover_jobs" => generation::recover(&input["document"], rows(&input["jobs"])),
        "generation_outcomes" => Ok(
            json!({"generationTasks":generation::outcomes(&input["document"],rows(&input["taskKeys"]))}),
        ),
        "capabilities" => super::generation_parameters::contract(&input["model"]),
        _ => anyhow::bail!("Unknown domain action"),
    };
    Ok(match result {
        Ok(mut v) => {
            normalize_numbers(&mut v);
            v
        }
        Err(error) => {
            let r = error.downcast_ref::<Rejection>();
            let mut value = json!({"error":model_error(&error.to_string()),"code":r.map(|r|r.code).unwrap_or("DOMAIN_VALIDATION"),"stage":"validation","outcome":"not_executed","recovery":{"action":"correct_arguments"}});
            if let Some(r) = r {
                value["context"] = r.context.clone();
            }
            value
        }
    })
}

#[cfg(test)]
mod parity_tests {
    use super::*;
    fn differences(a: &Value, b: &Value, path: &str, out: &mut Vec<String>) {
        if equal(a, b) {
            return;
        }
        match (a, b) {
            (Value::Object(a), Value::Object(b)) => {
                let keys: std::collections::BTreeSet<_> = a.keys().chain(b.keys()).collect();
                for k in keys {
                    match (a.get(k), b.get(k)) {
                        (Some(a), Some(b)) => differences(a, b, &format!("{path}/{k}"), out),
                        (a, b) => out.push(format!("{path}/{k}: expected {a:?}, got {b:?}")),
                    }
                }
            }
            (Value::Array(a), Value::Array(b)) if a.len() == b.len() => {
                for (i, (a, b)) in a.iter().zip(b).enumerate() {
                    differences(a, b, &format!("{path}/{i}"), out)
                }
            }
            _ => {
                if out.len() < 12 {
                    out.push(format!("{path}: expected {a}, got {b}"));
                }
            }
        }
    }
    #[test]
    fn native_rules_match_contract_examples() {
        let cases: Vec<Value> = serde_json::from_str(include_str!("parity_cases.json")).unwrap();
        let mut failures = Vec::new();
        for case in cases {
            TEST_SEQUENCE.with(|v| v.set(Some(0)));
            let actual = execute(case["input"].clone());
            TEST_SEQUENCE.with(|v| v.set(None));
            match actual {
                Ok(actual) => {
                    let mut diff = Vec::new();
                    differences(&case["expected"], &actual, "", &mut diff);
                    if !diff.is_empty() {
                        failures.push(format!("{}:\n{}", text(&case["name"]), diff.join("\n")));
                    }
                }
                Err(e) => failures.push(format!("{}: {e}", text(&case["name"]))),
            }
        }
        assert!(failures.is_empty(), "{}", failures.join("\n\n"));
    }
}
