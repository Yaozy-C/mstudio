use super::*;
pub async fn recover(
    model: &impl CompletionModel,
    profile: &Profile,
    host: &impl Host,
    session: &mut Session,
    overflow: bool,
) -> Result<bool, String> {
    let before = raw_pressure(session, host);
    super::super::session::offload_old_images(session, host)?;
    prune_tool_results(session, host)?;
    let result = if overflow {
        compact_manual(model, profile, host, session).await
    } else if over_budget(session, profile, host) {
        compact(model, profile, host, session).await
    } else {
        Ok(false)
    };
    if host.token().is_cancelled() {
        return Err(crate::app_error::cancelled());
    }
    if let Err(error) = result {
        host.record(
            "compaction/warning",
            json!({"reason":error,"trigger":if overflow {"context-overflow"} else {"pressure"}}),
        )?;
    }
    Ok(raw_pressure(session, host) < before)
}

/// Keep the newest messages intact and only replace a balanced historical prefix.
pub(super) fn compact_end(messages: &[Message], max_source: usize, retain: usize) -> Option<usize> {
    if messages.len() < 5 {
        return None;
    }
    let mut cost = 0;
    let mut pending = 0isize;
    let mut end = None;
    for (index, message) in messages
        .iter()
        .enumerate()
        .skip(1)
        .take(super::super::session::consumed_prefix(messages).saturating_sub(1))
    {
        cost += tokens(message);
        if cost > max_source {
            break;
        }
        match message {
            Message::Assistant { content, .. } => {
                pending += content
                    .iter()
                    .filter(|c| matches!(c, AssistantContent::ToolCall(_)))
                    .count() as isize
            }
            Message::User { content } => {
                pending -= content
                    .iter()
                    .filter(|c| matches!(c, UserContent::ToolResult(_)))
                    .count() as isize
            }
            _ => {}
        }
        let tail_cost: usize = messages[index + 1..].iter().map(tokens).sum();
        if pending == 0
            && tail_cost >= retain
            && (matches!(message, Message::Assistant { .. })
                || matches!(message, Message::User { content } if content.iter().any(|c| matches!(c, UserContent::ToolResult(_)))))
        {
            end = Some(index + 1);
        }
    }
    end.filter(|end| *end > 2)
}

pub async fn compact(
    model: &impl CompletionModel,
    profile: &Profile,
    host: &impl Host,
    session: &mut Session,
) -> Result<bool, String> {
    let retain = context_window(session, profile).map_or(0, |n| n.saturating_mul(16) / 100);
    compact_with_retain(model, profile, host, session, retain).await
}
pub async fn compact_manual(
    model: &impl CompletionModel,
    profile: &Profile,
    host: &impl Host,
    session: &mut Session,
) -> Result<bool, String> {
    compact_with_retain(model, profile, host, session, 0).await
}
async fn compact_with_retain(
    model: &impl CompletionModel,
    profile: &Profile,
    host: &impl Host,
    session: &mut Session,
    retain: usize,
) -> Result<bool, String> {
    let Some(end) = compact_end(
        &session.messages,
        input_budget(session, profile)
            .unwrap_or(usize::MAX)
            .saturating_sub(tokens(&session.messages[0])),
        retain,
    ) else {
        return Ok(false);
    };
    let old = session.messages[1..end].to_vec();
    host.record(
        "compaction/start",
        json!({"from":1,"to":end,"estimatedTokens":old.iter().map(tokens).sum::<usize>()}),
    )?;
    let mut history = vec![session.messages[0].clone()];
    history.extend(old.iter().cloned());
    history.push(Message::user("Summarize the earlier conversation into concise continuation context. Preserve the original current user request, goal, target/task IDs, permitted scope and completion criteria. Distinguish explicit user decisions, existing designs and assistant proposals; do not turn historical todos into current tasks. Retain useful read locations (objects, fields, pages), findings, revisions, asset IDs and retrievable tool-result references (turnId/callId). Do not copy stale project snapshots; fresh project state is supplied separately. For images preserve actual observations and reopenable references, without substituting text for pixel verification. Record saved actions, results, exact failure causes, missing evidence and next steps to avoid repeated research or writes. Do not repeat complete rules still present; summarize removed rules only as needed and do not claim their full text remains available. Remove unrelated shot todos, duplicate outputs and long discussion. Add no goals, assumptions or invented IDs. Preserve quoted user wording and requested content language. Output only the summary; execute no tools."));
    crate::assistant::harness::context_source::wire(&mut history);
    let request = CompletionRequest {
        model: None,
        preamble: None,
        chat_history: history,
        documents: vec![],
        tools: host.definitions(),
        temperature: None,
        max_tokens: Some(8192),
        tool_choice: Some(rig_core::message::ToolChoice::None),
        additional_params: None,
        output_schema: None,
        record_telemetry_content: false,
    };
    let response = match tokio::select! {
        result = model.completion(request) => result,
        _ = host.token().cancelled() => { host.record("compaction/end", json!({"status":"cancelled"}))?; return Err(crate::app_error::cancelled()); },
        _ = tokio::time::sleep(std::time::Duration::from_secs(240)) => { host.record("compaction/end", json!({"status":"timeout"}))?; return Err("压缩旧对话超时；已完成操作保留".into()); },
    } {
        Ok(response) => response,
        Err(error) => {
            host.record("compaction/end", json!({"status":"failed"}))?;
            return Err(crate::assistant::failure::describe(&error.into(), ""));
        }
    };
    let summary: String = response
        .choice
        .iter()
        .filter_map(|c| match c {
            AssistantContent::Text(t) => Some(t.text.as_str()),
            _ => None,
        })
        .collect();
    if summary.trim().is_empty()
        || tokens(&Message::user(summary.clone())) >= old.iter().map(tokens).sum::<usize>()
    {
        host.record("compaction/end", json!({"status":"no-reduction"}))?;
        return Ok(false);
    }
    let replacement = Message::user(format!(
        "<compacted-summary>\n{}\n</compacted-summary>",
        summary.trim()
    ));
    session.replace_range(host, 1, end, replacement)?;
    host.record("compaction/end", json!({"status":"completed","inputTokens":response.usage.input_tokens,"outputTokens":response.usage.output_tokens,"totalTokens":response.usage.total_tokens,"cachedInputTokens":response.usage.cached_input_tokens}))?;
    Ok(true)
}
