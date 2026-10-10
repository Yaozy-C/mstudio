//! Prompt preparation with explicitly referenced context; no implicit history or edits.
use crate::database::Store;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Request {
    project_id: String,
    media_model_id: String,
    kind: String,
    prompt: String,
    parameters: Value,
    inputs: Vec<Input>,
    #[serde(default)]
    context_references: Vec<super::attachments::Reference>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Input {
    asset_id: String,
    role: String,
    purpose: String,
    start: Option<f64>,
    end: Option<f64>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Prepared {
    prompt: String,
    references: Vec<Purpose>,
}
#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Purpose {
    asset_id: String,
    purpose: String,
}
#[path = "media_prompt_rules.rs"]
mod rules;

fn parse_reply(raw: &str, inputs: &[Input]) -> Result<Prepared, String> {
    let raw = raw.trim();
    let body = if let Some(rest) = raw.strip_prefix("```") {
        rest.strip_prefix("json")
            .unwrap_or(rest)
            .trim()
            .strip_suffix("```")
            .unwrap_or(rest)
            .trim()
    } else {
        raw
    };
    let value: Value = serde_json::from_str(body)
        .map_err(|_| "提示词整理返回格式无效，请重试；尚未提交生成任务")?;
    if let Some(error) = value.get("error").and_then(Value::as_str) {
        return Err(format!(
            "提示词存在未解决的冲突：{}；尚未提交生成任务",
            error.trim()
        ));
    }
    let result: Prepared = serde_json::from_value(value)
        .map_err(|_| "提示词整理返回格式无效，请重试；尚未提交生成任务")?;
    if result.prompt.trim().is_empty() || result.prompt.chars().count() > 12000 {
        return Err("整理后的提示词为空或过长，请重试".into());
    }
    if result.references.len() != inputs.len()
        || result.references.iter().zip(inputs).any(|(r, i)| {
            r.asset_id != i.asset_id
                || r.purpose.trim().is_empty()
                || r.purpose.chars().count() > 1000
        })
    {
        return Err("提示词整理改变了引用素材，请重试；尚未提交生成任务".into());
    }
    Ok(result)
}

#[tauri::command]
pub async fn prepare_media_prompt(
    app: tauri::AppHandle,
    store: State<'_, Store>,
    request: Request,
) -> Result<Prepared, String> {
    if !["image", "video"].contains(&request.kind.as_str())
        || request.prompt.trim().is_empty()
        || request.prompt.chars().count() > 12000
    {
        return Err("生成描述或素材数量无效".into());
    }
    super::skills::initialize(&app).map_err(|e| e.to_string())?;
    let (model, key, document, guidance) = {
        let db = store.db.lock().unwrap();
        let (model, key) = crate::models::resolve(&db, Some(&request.project_id), None)
            .map_err(|e| e.to_string())?;
        let document: String = db
            .query_row(
                "SELECT document FROM projects WHERE id=?1",
                [&request.project_id],
                |r| r.get(0),
            )
            .map_err(|e| e.to_string())?;
        let selected = crate::models::media::read(&db)
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|m| m.id == request.media_model_id && m.enabled && m.kind == request.kind)
            .ok_or("所选生成模型不可用，请重新选择")?;
        let author =
            super::prompt_guidance::quick_profile(&db, &request.kind).map_err(|e| e.to_string())?;
        let doc: Value = serde_json::from_str(&document).map_err(|e| e.to_string())?;
        let production = json!({"task":{"kind":request.kind,"modelId":selected.id}});
        let guidance = super::prompt_guidance::for_quick(
            &db,
            &author,
            &request.kind,
            &request.prompt,
            &production,
            &doc,
        )
        .map_err(|e| e.to_string())?;
        (model, key, document, guidance)
    };
    let doc: Value = serde_json::from_str(&document).map_err(|e| e.to_string())?;
    let mut refs = vec![];
    let mut metadata = vec![];
    let mut ids = std::collections::HashSet::new();
    for (index, input) in request.inputs.iter().enumerate() {
        let asset = doc["assets"]
            .as_array()
            .and_then(|a| a.iter().find(|a| a["id"] == input.asset_id))
            .ok_or("引用素材已移除")?;
        if !["image", "video"].contains(&asset["kind"].as_str().unwrap_or(""))
            || !ids.insert((&input.asset_id, &input.role))
            || ![
                "reference",
                "edit",
                "first-frame",
                "last-frame",
                "video-reference",
            ]
            .contains(&input.role.as_str())
            || input.purpose.chars().count() > 1000
        {
            return Err("引用素材或用途无效".into());
        }
        refs.push(super::attachments::Reference {
            kind: "asset".into(),
            id: input.asset_id.clone(),
        });
        metadata.push(
            json!({"index":index+1,"assetId":input.asset_id,"role":input.role,
            "purpose":input.purpose,"start":input.start,"end":input.end}),
        );
    }
    refs.extend(request.context_references.iter().cloned());
    let description =
        serde_json::to_string(&json!({"kind":request.kind,"description":request.prompt,
        "parameters":request.parameters,"references":metadata}))
        .map_err(|e| e.to_string())?;
    let payload = super::attachments::payload(&store, &doc, &description, &refs, &model.profile)
        .map_err(|e| format!("无法整理提示词：{e}"))?;
    let messages = json!([{"role":"system","content":format!("{}\n{guidance}", rules::system(&request.kind))},{"role":"user","content":payload}]);
    let reply = tokio::time::timeout(
        std::time::Duration::from_secs(120),
        super::agent::complete_with_resume(&model.profile, &key, messages, None, None, None),
    )
    .await
    .map_err(|_| "提示词整理超时，请重试；尚未提交生成任务")??;
    parse_reply(&reply, &request.inputs)
}

#[cfg(test)]
#[path = "media_prompt_tests.rs"]
mod tests;
