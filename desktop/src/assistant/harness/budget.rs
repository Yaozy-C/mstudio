use super::{Host, session::Session};
use crate::assistant::config::Profile;
use rig_core::{
    completion::{CompletionModel, CompletionRequest},
    message::{AssistantContent, Message, ToolResultContent, UserContent},
};
use serde_json::json;

/// DSH fixed-density estimate uses UTF-16 length, not UTF-8 bytes.
fn text_tokens(text: &str) -> usize {
    text.encode_utf16().count().div_ceil(4)
}
fn structural<T: serde::Serialize>(value: &T) -> usize {
    serde_json::to_string(value).map_or(0, |v| text_tokens(&v)) + 4
}
fn result_tokens(part: &ToolResultContent) -> usize {
    match part {
        ToolResultContent::Text(t) => text_tokens(&t.text) + 4,
        // Rig has no route-owned image tokenizer. Keep an explicit estimate;
        // never count base64 bytes as textual tokens.
        _ => 1024,
    }
}
pub(crate) fn tokens(message: &Message) -> usize {
    use rig_core::message::ReasoningContent;
    match message {
        Message::System { content } => text_tokens(content) + 4,
        Message::User { content } => {
            4 + content
                .iter()
                .map(|part| match part {
                    UserContent::Text(t) => text_tokens(&t.text) + 4,
                    UserContent::ToolResult(r) => {
                        4 + r.content.iter().map(result_tokens).sum::<usize>()
                    }
                    _ => 1024,
                })
                .sum::<usize>()
        }
        Message::Assistant { content, .. } => {
            4 + content
                .iter()
                .map(|part| match part {
                    AssistantContent::Text(t) => text_tokens(&t.text) + 4,
                    AssistantContent::ToolCall(c) => {
                        text_tokens(&c.function.name) + structural(&c.function.arguments)
                    }
                    AssistantContent::Reasoning(r) => r
                        .content
                        .iter()
                        .map(|part| match part {
                            ReasoningContent::Text { text, .. }
                            | ReasoningContent::Summary(text) => text_tokens(text) + 4,
                            _ => structural(part),
                        })
                        .sum(),
                    AssistantContent::Image(_) => 1024,
                })
                .sum::<usize>()
        }
    }
}
#[derive(Clone, Debug, PartialEq)]
pub struct UsageAnchor {
    header: serde_json::Value,
    estimated: usize,
    baseline: usize,
}
pub fn header(
    profile: &Profile,
    tools: &[rig_core::completion::ToolDefinition],
) -> serde_json::Value {
    json!({"route":profile,"tools":tools,"maxTokens":if profile.adapter == "anthropic-native" {Some(8192)} else {None},"store":if profile.adapter == "openai-responses" {Some(false)} else {None}})
}
/// Anchor AFTER assistant commit, so its output is neither omitted nor counted twice.
/// Provider usage is normalized separately; cache counters have adapter-specific semantics.
#[cfg(test)]
pub fn anchor(session: &mut Session, profile: &Profile, host: &impl Host, input: u64, output: u64) {
    anchor_request(
        session,
        header(profile, &host.definitions()),
        input.saturating_add(output),
    );
}
/// OpenAI cache counts are a subset of input; Anthropic reports disjoint buckets.
/// Prefer the adapter's full total, also covering providers that only report a total.
pub fn usage_total(profile: &Profile, usage: &rig_core::completion::Usage) -> u64 {
    let base = usage.input_tokens.saturating_add(usage.output_tokens);
    let base = if profile.adapter == "anthropic-native" {
        base.saturating_add(usage.cached_input_tokens)
            .saturating_add(usage.cache_creation_input_tokens)
    } else {
        base
    };
    base.max(usage.total_tokens)
}
pub fn anchor_request(session: &mut Session, request_header: serde_json::Value, usage: u64) {
    let tools = &request_header["tools"];
    let estimated = session.messages.iter().map(tokens).sum::<usize>()
        + if tools.as_array().is_none_or(|v| v.is_empty()) {
            0
        } else {
            structural(tools)
        };
    session.usage_anchor = Some(UsageAnchor {
        header: request_header,
        estimated,
        baseline: estimated.max(usage as usize),
    });
}
pub fn pressure(session: &Session, profile: &Profile, host: &impl Host) -> usize {
    let estimated = raw_pressure(session, host);
    match &session.usage_anchor {
        Some(anchor) if anchor.header == header(profile, &host.definitions()) => anchor
            .baseline
            .saturating_add(estimated)
            .saturating_sub(anchor.estimated),
        _ => estimated,
    }
}
pub fn raw_pressure(session: &Session, host: &impl Host) -> usize {
    let tools = host.definitions();
    session.messages.iter().map(tokens).sum::<usize>()
        + if tools.is_empty() {
            0
        } else {
            structural(&tools)
        }
}

/// Proactive failure is diagnostic, not a provider context-window error.
/// Overflow retry is permitted only after a durable, measured reduction.
pub async fn recover(
    model: &impl CompletionModel,
    profile: &Profile,
    host: &impl Host,
    session: &mut Session,
    overflow: bool,
) -> Result<bool, String> {
    let before = raw_pressure(session, host);
    super::session::offload_old_images(session, host)?;
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
fn compact_end(messages: &[Message], max_source: usize, retain: usize) -> Option<usize> {
    if messages.len() < 5 {
        return None;
    }
    let mut cost = 0;
    let mut pending = 0isize;
    let mut end = None;
    for (index, message) in messages.iter().enumerate().skip(1).take(messages.len() - 3) {
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
    history.push(Message::user("请把以上较早的对话压缩为可直接继续执行的简短摘要。优先保留：本轮用户原话与目标、目标对象/任务 ID 和允许修改范围、完成条件；区分用户明确决定、已有设计和助手候选判断，不把历史待办升级为当前任务。保留已读取资料的对象/字段/分页位置与关键结论、相关版本和素材 ID；图片仅保留实际观察及可重新读取的引用，不用文字结论代替像素验证。记录已保存操作及结果、失败调用的具体原因、尚缺的证据和下一步，避免重做调查或重复写入。完整且仍在上下文中的规则无需复述；被移除的规则只概括本任务需要的要点，不声称全文仍可用。删去无关镜头待办、重复工具输出和冗长讨论，不添加目标、推断或新对象 ID。只输出摘要，不执行工具。"));
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
    host.record("compaction/end", json!({"status":"completed","inputTokens":response.usage.input_tokens,"outputTokens":response.usage.output_tokens}))?;
    Ok(true)
}

pub fn prune_tool_results(session: &mut Session, host: &impl Host) -> Result<bool, String> {
    let mut changed = false;
    for index in 0..session.messages.len().saturating_sub(2) {
        let mut message = session.messages[index].clone();
        let Message::User { content } = &mut message else {
            continue;
        };
        let mut pruned = false;
        for part in content {
            let UserContent::ToolResult(result) = part else {
                continue;
            };
            for item in &mut result.content {
                let ToolResultContent::Text(text) = item else {
                    continue;
                };
                if text.text.chars().count() <= 6000 {
                    continue;
                }
                let head: String = text.text.chars().take(2500).collect();
                let tail: String = text
                    .text
                    .chars()
                    .rev()
                    .take(1000)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                text.text = format!("{head}\n[较早工具结果中间内容已省略，可重新 inspect]\n{tail}");
                pruned = true;
            }
        }
        if pruned {
            session.replace_range(host, index, index + 1, message)?;
            changed = true;
        }
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rig_core::message::{ToolCall, ToolFunction};
    use serde_json::json;
    #[test]
    fn compaction_never_cuts_a_tool_call_from_its_result() {
        let tool = ToolCall::from_wire(
            "call-1",
            ToolFunction {
                name: "read".into(),
                arguments: json!({}),
            },
        );
        let messages = vec![
            Message::System {
                content: "rules".into(),
            },
            Message::user("first"),
            Message::Assistant {
                id: None,
                content: vec![AssistantContent::text("done")],
            },
            Message::Assistant {
                id: None,
                content: vec![AssistantContent::ToolCall(tool.clone())],
            },
            super::super::session::result_message(&tool, &json!({"ok":true})),
            Message::user("current"),
            Message::Assistant {
                id: None,
                content: vec![AssistantContent::text("reply")],
            },
        ];
        assert_eq!(compact_end(&messages, 100_000, 0), Some(5));
        assert_eq!(
            compact_end(&messages, tokens(&messages[1]) + tokens(&messages[2]), 0),
            Some(3)
        );
    }
    #[test]
    fn transport_bytes_are_not_text_tokens() {
        let image = Message::User {
            content: vec![
                UserContent::text("look"),
                UserContent::image_url(
                    format!("data:image/jpeg;base64,{}", "a".repeat(500_000)),
                    None,
                    None,
                ),
            ],
        };
        assert!(tokens(&image) < 5000);
        assert!(tokens(&Message::user("x".repeat(200_000))) > 40_000);
        assert_eq!(text_tokens("中文中文"), 1);
        assert_eq!(text_tokens("😀😀"), 1);
    }
}
