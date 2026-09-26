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
    let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(20 * 60);
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
    for index in 1..=16 {
        if host.token().is_cancelled() {
            return Err("已停止回答；已输出内容和已完成操作保留".into());
        }
        session.messages.extend(host.injected()?);
        if budget::pressure(&session, profile, host) > profile.input_budget() {
            budget::recover(model, profile, host, &mut session, false).await?;
        }

        host.record(
            "step/start",
            json!({"step":index,"historyMessages":session.messages.len()}),
        )?;
        let mut result = step(model, profile, host, &mut session, streaming, key, deadline).await;
        if matches!(&result, Err(error) if error == "CONTEXT_WINDOW_EXCEEDED") {
            if budget::recover(model, profile, host, &mut session, true).await? {
                result = step(model, profile, host, &mut session, streaming, key, deadline).await;
            }
            if matches!(&result, Err(error) if error == "CONTEXT_WINDOW_EXCEEDED") {
                result = Err("该模型报告上下文窗口已满；当前输入无法继续压缩，请减少本轮附件或调整模型窗口设置".into());
            }
        }
        host.record(
            "step/end",
            json!({"step":index,"status":if result.is_ok() {"completed"} else {"failed"}}),
        )?;
        let finished = result?;
        if finished {
            return Ok(session.text);
        }
    }
    Err("本轮已达到 16 个执行步骤；已完成操作保留，可继续下一轮".into())
}
async fn step(
    model: &impl CompletionModel,
    profile: &Profile,
    host: &impl Host,
    session: &mut Session,
    streaming: bool,
    key: &str,
    deadline: tokio::time::Instant,
) -> Result<bool, String> {
    let request = CompletionRequest {
        model: None,
        preamble: None,
        chat_history: session.messages.clone(),
        documents: vec![],
        tools: host.definitions(),
        temperature: None,
        max_tokens: (profile.adapter == "anthropic-native").then_some(8192),
        tool_choice: None,
        additional_params: (profile.adapter == "openai-responses").then(|| json!({"store":false})),
        output_schema: None,
        record_telemetry_content: false,
    };
    let request_header = budget::header(profile, &request.tools);
    let response = model::request(model, request, host, session, streaming, key, deadline).await?;
    let usage = budget::usage_total(profile, &response.usage);
    host.record("request/usage", json!({"inputTokens":response.usage.input_tokens,"outputTokens":response.usage.output_tokens,"totalTokens":response.usage.total_tokens,"cachedInputTokens":response.usage.cached_input_tokens}))?;
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
    // Preserve the full typed message: provider signatures, reasoning handles and call IDs.
    session.append(
        host,
        Message::Assistant {
            id: response.message_id,
            content: response.choice,
        },
    )?;
    budget::anchor_request(session, request_header, usage);
    if let Some(reason) = stop_reason {
        host.record("model/stop", json!({"stopReason":reason}))?;
        scheduler::skip(
            host,
            session,
            &calls,
            "MODEL_OUTPUT_INCOMPLETE",
            "模型输出不完整，此调用未执行",
        )?;
        return Err(if reason == "refusal" {
            "模型拒绝了本次请求；未执行本次工具调用"
        } else {
            "模型输出被截断；已输出内容保留，未执行本次不完整的工具调用"
        }
        .into());
    }
    if calls.is_empty() {
        if session.text.trim().is_empty() {
            return Err("模型回答为空".into());
        }
        return Ok(true);
    }
    scheduler::execute(host, session, &calls).await?;
    if !session.text.is_empty() && !session.text.ends_with("\n\n") {
        session.publish(host, "\n\n")?;
    }
    Ok(false)
}
