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
    } else if pressure(session, profile, host) > profile.input_budget() {
        compact(model, profile, host, session).await
    } else {
        Ok(false)
    };
    if host.token().is_cancelled() {
        return Err("已停止回答；已完成操作保留".into());
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
    compact_with_retain(model, profile, host, session, profile.retain_budget()).await
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
        profile
            .input_budget()
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
    history.push(Message::user("请把以上较早的对话压缩为可直接继续执行的简短摘要。优先保留：本轮用户原话与目标、目标对象/任务 ID 和允许修改范围、完成条件；区分用户明确决定、已有设计和助手候选判断，不把历史待办升级为当前任务。保留已读取资料的对象/字段/分页位置与关键结论、相关版本、素材 ID 和可重新读取的工具结果引用（turnId/callId）。不要复制已过时的工程快照；当前工程摘要由程序提供，历史只保留必要决定和变化。图片仅保留实际观察及可重新读取的引用，不用文字结论代替像素验证。记录已保存操作及结果、失败调用的具体原因、尚缺的证据和下一步，避免重做调查或重复写入。完整且仍在上下文中的规则无需复述；被移除的规则只概括本任务需要的要点，不声称全文仍可用。删去无关镜头待办、重复工具输出和冗长讨论，不添加目标、推断或新对象 ID。只输出摘要，不执行工具。"));
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
        _ = host.token().cancelled() => { host.record("compaction/end", json!({"status":"cancelled"}))?; return Err("已停止回答；已完成操作保留".into()); },
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
