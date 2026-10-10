use super::{Host, budget, model, scheduler, session::Session};
use crate::assistant::config::Profile;
use rig_core::{
    completion::{CompletionModel, CompletionRequest, FinishReason},
    message::{AssistantContent, Message},
};
use serde_json::json;

pub async fn run(
    model: &impl CompletionModel,
    profile: &Profile,
    host: &impl Host,
    session: Session,
    streaming: bool,
    key: &str,
) -> Result<String, String> {
    let deadline = host
        .deadline()
        .unwrap_or_else(|| tokio::time::Instant::now() + std::time::Duration::from_secs(20 * 60));
    run_until(model, profile, host, session, streaming, key, deadline).await
}
pub async fn run_until(
    model: &impl CompletionModel,
    profile: &Profile,
    host: &impl Host,
    mut session: Session,
    streaming: bool,
    key: &str,
    deadline: tokio::time::Instant,
) -> Result<String, String> {
    let mut index = 0_u64;
    loop {
        index += 1;
        if host.token().is_cancelled() {
            return Err(crate::app_error::cancelled());
        }
        session.messages.extend(host.injected()?);
        if budget::over_budget(&session, profile, host) {
            budget::recover(model, profile, host, &mut session, false).await?;
        }

        host.record(
            "step/start",
            json!({"step":index,"historyMessages":session.messages.len()}),
        )?;
        let mut result = step(model, profile, host, &mut session, streaming, key, deadline).await;
        if matches!(&result, Err(model::RequestError::ContextOverflow)) {
            if budget::recover(model, profile, host, &mut session, true).await? {
                result = step(model, profile, host, &mut session, streaming, key, deadline).await;
            }
            if matches!(&result, Err(model::RequestError::ContextOverflow)) {
                result = Err("该模型报告上下文窗口已满；当前输入无法继续压缩，请减少本轮附件或调整模型窗口设置".into());
            }
        }
        host.record(
            "step/end",
            json!({"step":index,"status":if result.is_ok() {"completed"} else {"failed"}}),
        )?;
        if let Some(mut answer) = result.map_err(String::from)? {
            let notice = session.delegation_outcomes.notice();
            if !notice.is_empty() {
                session.publish(host, &notice)?;
                answer.push_str(&notice);
            }
            return Ok(answer);
        }
    }
}
async fn step(
    model: &impl CompletionModel,
    profile: &Profile,
    host: &impl Host,
    session: &mut Session,
    streaming: bool,
    key: &str,
    deadline: tokio::time::Instant,
) -> Result<Option<String>, model::RequestError> {
    super::metering::record(host, session, profile)?;
    let request = CompletionRequest {
        model: None,
        preamble: None,
        chat_history: session.messages.clone(),
        documents: vec![],
        tools: host.definitions(),
        temperature: None,
        max_tokens: (profile.adapter == "anthropic-native").then_some(8192),
        tool_choice: None,
        additional_params: match profile.adapter.as_str() {
            "openai-responses" => Some(json!({"store":false})),
            // Ask thinking-capable Gemini models for readable summaries without changing their budget.
            "gemini-native"
                if profile.model.rsplit('/').next().is_some_and(|model| {
                    model.starts_with("gemini-2.5-") || model.starts_with("gemini-3")
                }) =>
            {
                Some(json!({"generationConfig":{"thinkingConfig":{"includeThoughts":true}}}))
            }
            _ => None,
        },
        output_schema: None,
        record_telemetry_content: false,
    };
    let request_header = budget::header(profile, &request.tools);
    let response = model::request(model, request, host, session, streaming, key, deadline).await?;
    if !streaming {
        let mut thinking = super::thinking::Thinking::new();
        for (index, part) in response.choice.iter().enumerate() {
            if let AssistantContent::Reasoning(reasoning) = part {
                thinking.update(
                    host,
                    &index.to_string(),
                    &super::thinking::readable(reasoning),
                    true,
                )?;
            }
        }
        thinking.finish(host)?;
    }
    let usage = budget::usage_total(profile, &response.usage);
    host.record("request/usage", json!({"inputTokens":response.usage.input_tokens,"outputTokens":response.usage.output_tokens,"totalTokens":response.usage.total_tokens,"cachedInputTokens":response.usage.cached_input_tokens,"contextWindow":response.raw["tokenUsage"]["modelContextWindow"],"contextTokens":response.raw["tokenUsage"]["last"]["totalTokens"]}))?;
    let stop_reason = match response.finish_reason() {
        Some(FinishReason::Length) => Some("max-tokens"),
        Some(FinishReason::ContentFilter) => Some("refusal"),
        _ => None,
    };
    let calls: Vec<_> = response
        .choice
        .iter()
        .filter_map(|c| match c {
            AssistantContent::ToolCall(c) => Some(c.clone()),
            _ => None,
        })
        .collect();
    let mut ids = std::collections::HashSet::new();
    if calls.iter().any(|call| !ids.insert(call.id.as_str())) {
        return Err("模型返回重复的工具调用标识，本次操作未执行".into());
    }
    let text: String = response
        .choice
        .iter()
        .filter_map(|c| match c {
            AssistantContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    if !streaming && !text.is_empty() {
        session.publish(host, &text)?;
    }
    let final_text: String = response
        .choice
        .iter()
        .filter_map(|c| match c {
            AssistantContent::Text(t)
                if t.additional_params
                    .as_ref()
                    .and_then(|p| p.get("phase"))
                    .and_then(|v| v.as_str())
                    != Some("commentary") =>
            {
                Some(t.text.as_str())
            }
            _ => None,
        })
        .collect();
    let commentary: String = response
        .choice
        .iter()
        .filter_map(|c| match c {
            AssistantContent::Text(t)
                if t.additional_params
                    .as_ref()
                    .and_then(|p| p.get("phase"))
                    .and_then(|v| v.as_str())
                    == Some("commentary") =>
            {
                Some(t.text.as_str())
            }
            _ => None,
        })
        .collect();
    // Preserve the full typed message: provider signatures, reasoning handles and call IDs.
    session.append(
        host,
        Message::Assistant {
            id: response.message_id,
            content: response.choice,
        },
    )?;
    budget::anchor_response(session, profile, request_header, &response.raw, usage);
    if let Some(reason) = stop_reason {
        host.record("model/stop", json!({"stopReason":reason}))?;
        scheduler::skip(
            host,
            session,
            &calls,
            "MODEL_OUTPUT_INCOMPLETE",
            "Model output incomplete; this call was not executed",
        )?;
        return Err(if reason == "refusal" {
            "模型拒绝了本次请求；未执行本次工具调用"
        } else {
            "模型输出被截断；已输出内容保留，未执行本次不完整的工具调用"
        }
        .into());
    }
    if calls.is_empty() {
        if final_text.trim().is_empty() {
            return Err("模型回答为空".into());
        }
        if !commentary.is_empty() {
            host.record("assistant/progress", json!({"text":commentary}))?;
        }
        session.replace_visible(host, &final_text)?;
        return Ok(Some(final_text));
    }
    session.progress(host, &text)?;
    scheduler::execute(host, session, &calls).await?;
    Ok(None)
}
