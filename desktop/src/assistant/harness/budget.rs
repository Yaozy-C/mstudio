use super::{Host, session::Session};
use crate::assistant::config::Profile;
use rig_core::{
    completion::{CompletionModel, CompletionRequest},
    message::{AssistantContent, Message, ToolResultContent, UserContent},
};
use serde_json::json;

/// Shared fixed-density estimate; provider anchors calibrate actual totals.
fn text_tokens(text: &str) -> usize {
    super::metering::text_tokens(text)
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
    anchor_prefix(session, request_header, usage, session.messages.len());
}
fn anchor_prefix(
    session: &mut Session,
    request_header: serde_json::Value,
    usage: u64,
    message_count: usize,
) {
    let tools = &request_header["tools"];
    let estimated = session.messages[..message_count]
        .iter()
        .map(tokens)
        .sum::<usize>()
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
pub fn anchor_response(
    session: &mut Session,
    profile: &Profile,
    request_header: serde_json::Value,
    raw: &serde_json::Value,
    usage: u64,
) {
    if profile.adapter != "codex" {
        anchor_request(session, request_header, usage);
        return;
    }
    let report = &raw["tokenUsage"];
    if let Some(window) = report["modelContextWindow"].as_u64().filter(|n| *n > 0) {
        session.provider_context_window = usize::try_from(window).ok();
    }
    // Native usage can arrive after a tool result. Anchor to the measured prefix,
    // and never use cumulative billing totals as the current context size.
    if let Some(usage) = report["last"]["totalTokens"].as_u64().filter(|n| *n > 0) {
        let count = raw["contextMessageCount"]
            .as_u64()
            .and_then(|n| usize::try_from(n).ok())
            .unwrap_or(session.messages.len());
        if count <= session.messages.len() {
            anchor_prefix(session, request_header, usage, count);
        }
    }
}
pub fn context_window(session: &Session, profile: &Profile) -> Option<usize> {
    match (profile.context_window(), session.provider_context_window) {
        (Some(configured), Some(reported)) => Some(configured.min(reported)),
        (configured, reported) => configured.or(reported),
    }
}
pub fn input_budget(session: &Session, profile: &Profile) -> Option<usize> {
    context_window(session, profile).map(|n| n.saturating_mul(4) / 5)
}
pub fn over_budget(session: &Session, profile: &Profile, host: &impl Host) -> bool {
    input_budget(session, profile).is_some_and(|limit| pressure(session, profile, host) > limit)
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
pub fn prune_tool_results(session: &mut Session, host: &impl Host) -> Result<bool, String> {
    let mut changed = false;
    for index in 0..super::session::consumed_prefix(&session.messages) {
        let mut message = session.messages[index].clone();
        let Message::User { content } = &mut message else {
            continue;
        };
        let mut pruned = false;
        for part in content {
            let UserContent::ToolResult(result) = part else {
                continue;
            };
            // Activated instructions must remain readable, not turn into a head/tail fragment.
            if result.name == "mstudio_read_skill" {
                continue;
            }
            for item in &mut result.content {
                let ToolResultContent::Text(text) = item else {
                    continue;
                };
                if text.text.chars().count() <= 8192 {
                    continue;
                }
                let head: String = text.text.chars().take(4096).collect();
                let tail: String = text
                    .text
                    .chars()
                    .rev()
                    .take(1024)
                    .collect::<String>()
                    .chars()
                    .rev()
                    .collect();
                text.text = format!(
                    "{head}\n[Middle of an earlier tool result omitted; inspect again if needed]\n{tail}"
                );
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
#[path = "tests/budget.rs"]
mod tests;

#[path = "compaction.rs"]
mod compaction;
#[cfg(test)]
use compaction::compact_end;
pub use compaction::{compact_manual, recover};
