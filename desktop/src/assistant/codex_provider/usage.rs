//! Codex reports cumulative billing and the last request's context separately.
use super::*;
use rig_core::completion::Usage;

#[derive(Default)]
pub(super) struct NativeUsage {
    report: Value,
    billed: Usage,
    pending_prefix: Option<usize>,
    measured_prefix: Option<usize>,
    current_reported: bool,
    changed: bool,
}
impl NativeUsage {
    pub fn continue_after_tool(&mut self, history_len: usize) {
        // Tool-call usage normally arrives only after its result is sent back.
        // The measured context ends at that assistant call, before the result.
        self.pending_prefix = (!self.current_reported).then(|| history_len.saturating_sub(1));
        self.current_reported = false;
    }
    pub fn observe(&mut self, report: &Value) {
        let Some(total) = report["total"]["totalTokens"].as_u64() else {
            return;
        };
        if total <= self.report["total"]["totalTokens"].as_u64().unwrap_or(0) {
            return;
        }
        self.measured_prefix = self.pending_prefix.take();
        self.current_reported = self.measured_prefix.is_none();
        self.report = report.clone();
        self.changed = true;
    }
    pub fn finish(&mut self, reason: FinishReason) -> CompletionResponse {
        let total = breakdown(&self.report["total"]);
        let delta = |current: Option<u64>, billed: Option<u64>| {
            current.map(|n| n.saturating_sub(billed.unwrap_or(0)))
        };
        let usage = if self.changed {
            Usage::new()
                .input_tokens(delta(total.input_tokens, self.billed.input_tokens))
                .output_tokens(delta(total.output_tokens, self.billed.output_tokens))
                .total_tokens(delta(total.total_tokens, self.billed.total_tokens))
                .cached_input_tokens(delta(
                    total.cached_input_tokens,
                    self.billed.cached_input_tokens,
                ))
                .cache_creation_input_tokens(delta(
                    total.cache_creation_input_tokens,
                    self.billed.cache_creation_input_tokens,
                ))
                .reasoning_tokens(delta(total.reasoning_tokens, self.billed.reasoning_tokens))
        } else {
            Usage::new()
        };
        self.billed = total;
        let raw = if std::mem::take(&mut self.changed) {
            json!({"tokenUsage":self.report,"contextMessageCount":self.measured_prefix})
        } else {
            Value::Null
        };
        CompletionResponse::new(
            vec![],
            usage,
            rig_core::message::Origin::new("mstudio.codex", "codex", ""),
            raw,
        )
        .with_finish_reason(reason)
    }
}
fn breakdown(value: &Value) -> Usage {
    Usage::new()
        .input_tokens(value["inputTokens"].as_u64())
        .output_tokens(value["outputTokens"].as_u64())
        .total_tokens(value["totalTokens"].as_u64())
        .cached_input_tokens(value["cachedInputTokens"].as_u64())
        .cache_creation_input_tokens(value["cacheWriteInputTokens"].as_u64())
        .reasoning_tokens(value["reasoningOutputTokens"].as_u64())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn report(input: u64, total: u64) -> Value {
        json!({"modelContextWindow":258400,
            "last":{"inputTokens":input,"outputTokens":20,"totalTokens":input+20},
            "total":{"inputTokens":total-20,"outputTokens":20,"totalTokens":total}})
    }
    #[test]
    fn delayed_reports_anchor_before_the_tool_result_and_bill_each_token_once() {
        let mut usage = NativeUsage::default();
        assert!(!usage.finish(FinishReason::ToolCalls).usage.is_reported());
        usage.continue_after_tool(5);
        usage.observe(&report(1000, 1020));
        let first = usage.finish(FinishReason::ToolCalls);
        assert_eq!(first.usage.total_tokens, Some(1020));
        assert_eq!(first.raw["contextMessageCount"], 4);
        usage.continue_after_tool(7);
        usage.observe(&report(1000, 1020)); // Duplicate notification must not consume the anchor.
        usage.observe(&report(1100, 2140));
        let second = usage.finish(FinishReason::ToolCalls);
        assert_eq!(second.usage.total_tokens, Some(1120));
        assert_eq!(second.raw["contextMessageCount"], 6);
        assert_eq!(second.raw["tokenUsage"]["last"]["totalTokens"], 1120);
        assert!(!usage.finish(FinishReason::ToolCalls).usage.is_reported());
    }
    #[test]
    fn reports_before_tool_calls_and_final_reports_use_the_current_context() {
        let mut usage = NativeUsage::default();
        usage.observe(&report(1000, 1020));
        assert!(usage.finish(FinishReason::ToolCalls).raw["contextMessageCount"].is_null());
        usage.continue_after_tool(5);
        usage.observe(&report(1100, 2140));
        let final_response = usage.finish(FinishReason::Stop);
        assert_eq!(final_response.usage.total_tokens, Some(1120));
        assert!(final_response.raw["contextMessageCount"].is_null());
    }
}
