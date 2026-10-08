//! Estimates explain composition; provider usage remains the billing authority.
use super::{Host, budget, session::Session};
use crate::assistant::config::Profile;
use rig_core::message::Message;
use serde_json::json;

pub fn text_tokens(text: &str) -> usize {
    // Same deterministic heuristic as the driver; actual provider usage calibrates pressure.
    text.encode_utf16().count().div_ceil(4)
}

pub fn record(host: &impl Host, session: &Session, profile: &Profile) -> Result<(), String> {
    let system: usize = session
        .messages
        .iter()
        .filter(|m| matches!(m, Message::System { .. }))
        .map(budget::tokens)
        .sum();
    let messages: usize = session
        .messages
        .iter()
        .filter(|m| !matches!(m, Message::System { .. }))
        .map(budget::tokens)
        .sum();
    let tools = text_tokens(&serde_json::to_string(&host.definitions()).unwrap_or_default());
    host.record("context/usage",json!({"estimatedSystemTokens":system,"estimatedMessageTokens":messages,"estimatedToolTokens":tools,"projectedTokens":budget::pressure(session,profile,host),"contextWindow":budget::context_window(session,profile),"messageCount":session.messages.len(),"note":"组成是估算，媒体使用估值；费用以 request/usage 和 compaction/end 的供应商用量为准"}))
}
