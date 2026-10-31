//! Resolve saved task settings directly into the native provider request.
use crate::models::{capabilities::Controls, media::MediaModel};
use anyhow::{Context, Result, ensure};
use serde_json::{Value, json};

// Model declarations were validated when saved. Preserve JSON Pointer array
// semantics here as well as nested objects and escaped property names.
pub(super) fn set(target: &mut Value, path: &str, value: Value) -> Result<()> {
    let segments: Vec<_> = path[1..]
        .split('/')
        .map(|s| s.replace("~1", "/").replace("~0", "~"))
        .collect();
    let mut current = target;
    for (i, segment) in segments.iter().enumerate() {
        let last = i + 1 == segments.len();
        if let Some(array) = current.as_array_mut() {
            let index: usize = segment.parse().context("JSON Pointer 数组索引无效")?;
            ensure!(
                index.to_string() == *segment && index <= array.len(),
                "JSON Pointer 数组索引无效或超出范围"
            );
            if index == array.len() {
                array.push(Value::Null);
            }
        }
        if last {
            if let Some(array) = current.as_array_mut() {
                array[segment.parse::<usize>()?] = value;
            } else {
                current
                    .as_object_mut()
                    .context("请求参数必须为对象")?
                    .insert(segment.clone(), value);
            }
            return Ok(());
        }
        let next = if current.is_array() {
            &mut current[segment.parse::<usize>()?]
        } else {
            current
                .as_object_mut()
                .context("请求参数必须为对象")?
                .entry(segment.clone())
                .or_insert(Value::Null)
        };
        if !next.is_object() && !next.is_array() {
            let segment = &segments[i + 1];
            *next = if segment
                .parse::<usize>()
                .is_ok_and(|n| n.to_string() == *segment)
            {
                json!([])
            } else {
                json!({})
            };
        }
        current = next;
    }
    Ok(())
}

pub(super) fn remove(target: &mut Value, path: &str) {
    let Some((parent, key)) = path.rsplit_once('/') else {
        return;
    };
    let Some(container) = target.pointer_mut(parent) else {
        return;
    };
    let key = key.replace("~1", "/").replace("~0", "~");
    if let Some(object) = container.as_object_mut() {
        object.remove(&key);
    } else if let Some(array) = container.as_array_mut()
        && let Ok(index) = key.parse::<usize>()
        && index.to_string() == key
        && index < array.len()
    {
        array.remove(index);
    }
}

pub(super) fn resolve(model: &MediaModel, task: &Value, document: &Value) -> Result<Value> {
    let controls = controls(model)?;
    resolve_with_controls(model, task, document, controls)
}

fn controls(model: &MediaModel) -> Result<Controls> {
    let defaults: Value = serde_json::from_str(include_str!(
        "../../../frontend/src/models/config/protocols.json"
    ))?;
    let controls: Controls = match model.capabilities.as_ref().and_then(|c| c.controls.clone()) {
        Some(controls) => controls,
        None => serde_json::from_value(
            defaults[&model.plugin]
                .get("controls")
                .cloned()
                .unwrap_or(json!({})),
        )?,
    };
    Ok(controls)
}

pub(super) fn contract(value: &Value) -> Result<Value> {
    let model: MediaModel = serde_json::from_value(value.clone())?;
    let controls = controls(&model)?;
    let mut properties = json!({});
    for (name, control, description) in [
        (
            "aspectRatio",
            &controls.aspect_ratio,
            "Output width:height ratio, e.g. 9:16. Use the selected model's enum; first-frame video may derive its ratio from the image.",
        ),
        (
            "resolution",
            &controls.resolution,
            "Output resolution label. Use the exact case-sensitive values in the selected model configuration.",
        ),
    ] {
        if let Some(control) = control
            && let Some(values) = &control.values
            && !values.is_empty()
        {
            properties[name] = json!({"type":"string","enum":values,"description":description});
        }
    }
    let numeric = |kind: &str, min: Option<f64>, max: Option<f64>, description: &str| {
        let mut value = json!({"type":kind,"description":description});
        if let Some(min) = min {
            value["minimum"] = json!(min);
        }
        if let Some(max) = max {
            value["maximum"] = json!(max);
        }
        value
    };
    if let Some(c) = &controls.duration {
        properties["duration"] = numeric(
            "number",
            c.min,
            c.max,
            "Generated source clip length in seconds. Use the configured model bounds when present. Editorial shot length is separate.",
        );
        properties["duration"]["exclusiveMinimum"] = json!(0);
    }
    if let Some(c) = &controls.image_size {
        for (key, description) in [
            (
                "width",
                "Image width in pixels. Supply height together; use the selected model configuration.",
            ),
            (
                "height",
                "Image height in pixels. Supply width together; use the selected model configuration.",
            ),
        ] {
            properties[key] = numeric("integer", Some(c.min.unwrap_or(1.)), c.max, description);
        }
    }
    let extra = crate::models::generation_options::declarations(&model)?;
    if !extra.is_empty() {
        let mut choices = json!({});
        for option in &extra {
            let mut schema = json!({"type":option.value_type,"description":option.label});
            if let Some(values) = &option.values {
                schema["enum"] = json!(values);
            }
            if let Some(min) = option.min {
                schema["minimum"] = json!(min);
            }
            if let Some(max) = option.max {
                schema["maximum"] = json!(max);
            }
            choices[&option.key] = schema;
        }
        properties["options"] = json!({"type":"object","properties":choices,"required":[],"additionalProperties":false});
    }
    let configured = |c: &Option<crate::models::capabilities::ControlDeclaration>| {
        c.as_ref()
            .and_then(|c| model.params.pointer(&c.path))
            .cloned()
            .unwrap_or(Value::Null)
    };
    let mut result = json!({"parameters":{"type":"object","properties":properties,"required":[],"additionalProperties":false,"description":"Explicit generation overrides. Read mstudio_models for the chosen model's supported fields, values and configuredDefaults. Omitted fields inherit task/configured settings; project export settings and shot duration do not imply these overrides."},"configuredDefaults":{"aspectRatio":configured(&controls.aspect_ratio),"resolution":configured(&controls.resolution),"duration":configured(&controls.duration),"imageSize":configured(&controls.image_size)},"omission":"Omitted fields inherit the referenced task, then configured defaults; remaining defaults are chosen by the provider."});
    if !extra.is_empty() {
        let mut defaults = json!({});
        for option in &extra {
            if let Some(value) = model.params.pointer(&option.path) {
                defaults[&option.key] =
                    crate::models::generation_options::normalized(option, value);
            }
        }
        result["configuredDefaults"]["options"] = defaults;
    }
    if controls.image_size.is_some() {
        result["constraints"] = json!(
            "Specify positive integer width and height together; use configured bounds when present."
        );
    }
    Ok(result)
}

fn resolve_with_controls(
    model: &MediaModel,
    task: &Value,
    document: &Value,
    controls: Controls,
) -> Result<Value> {
    let parameters = &task["parameters"];
    let mut options = model.params.clone();
    let declared = crate::models::generation_options::declarations(model)?;
    for (option, value) in
        crate::models::generation_options::validate_options(&parameters["options"], &declared)?
    {
        set(&mut options, &option.path, value.clone())?;
    }
    for (key, control, message) in [
        (
            "aspectRatio",
            &controls.aspect_ratio,
            "当前模型不支持这个比例，请重新设置",
        ),
        (
            "resolution",
            &controls.resolution,
            "当前模型不支持这个尺寸，请重新设置",
        ),
    ] {
        if let Some(control) = control
            && let Some(value) = parameters.get(key).filter(|v| v.as_str() != Some(""))
        {
            ensure!(
                control.values.as_ref().is_some_and(|values| value
                    .as_str()
                    .is_some_and(|s| values.iter().any(|v| v == s))),
                "{message}"
            );
            set(&mut options, &control.path, value.clone())?;
        }
    }
    if let Some(control) = &controls.duration
        && let Some(value) = parameters.get("duration")
    {
        let n = value
            .as_f64()
            .context("视频时长须为正数并符合配置中的范围")?;
        ensure!(
            n > 0.
                && control.min.is_none_or(|min| n >= min)
                && control.max.is_none_or(|max| n <= max),
            "视频时长须为正数并符合配置中的范围"
        );
        set(&mut options, &control.path, value.clone())?;
    }
    if let Some(control) = &controls.image_size
        && (parameters.get("width").is_some() || parameters.get("height").is_some())
    {
        for key in ["width", "height"] {
            ensure!(
                parameters[key].as_u64().is_some_and(|n| n > 0
                    && control.min.is_none_or(|min| n as f64 >= min)
                    && control.max.is_none_or(|max| n as f64 <= max)),
                "宽高须同时填写正整数并符合配置中的范围"
            );
        }
        set(
            &mut options,
            &control.path,
            json!({"width":parameters["width"],"height":parameters["height"]}),
        )?;
    }

    if model.plugin == "codex-image" {
        if options["size"].as_str().is_none_or(|s| s == "auto") {
            options["size"] = json!(codex_size(
                document["width"].as_f64().unwrap_or(0.),
                document["height"].as_f64().unwrap_or(0.)
            ));
        }
        if options["quality"].as_str().is_none_or(|s| s == "auto") {
            options["quality"] = json!("high");
        }
    }
    Ok(options)
}

fn codex_size(width: f64, height: f64) -> String {
    let ratio = width / height;
    if width <= 0. || height <= 0. || !(1. / 3. ..=3.).contains(&ratio) {
        return "2048x1152".into();
    }
    let sizes = [
        (1., "1024x1024"),
        (16. / 9., "2048x1152"),
        (9. / 16., "1152x2048"),
        (4. / 3., "2048x1536"),
        (3. / 4., "1536x2048"),
        (3. / 2., "1536x1024"),
        (2. / 3., "1024x1536"),
        (21. / 9., "2016x864"),
        (4. / 5., "1024x1280"),
        (5. / 4., "1280x1024"),
    ];
    let best = sizes
        .iter()
        .min_by(|a, b| {
            (ratio / a.0)
                .ln()
                .abs()
                .total_cmp(&(ratio / b.0).ln().abs())
        })
        .unwrap();
    if (ratio / best.0).ln().abs() <= 0.06 {
        return best.1.into();
    }
    let short = (2048. / ratio.max(1.) / 16.).round().max(1.) * 16.;
    let (mut w, mut h) = if ratio >= 1. {
        (2048., short)
    } else {
        (short, 2048.)
    };
    if w * h < 655_360. {
        let scale = (655_360. / (w * h)).sqrt();
        w = (w * scale / 16.).round() * 16.;
        h = (h * scale / 16.).round() * 16.;
    }
    format!("{}x{}", w.min(3840.), h.min(3840.))
}
