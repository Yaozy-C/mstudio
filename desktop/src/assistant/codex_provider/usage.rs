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
    pub fn finish(&mut self, reason: FinishReason) -> StreamFinal {
        let total = breakdown(&self.report["total"]);
        let delta = Usage {
            input_tokens: total.input_tokens.saturating_sub(self.billed.input_tokens),
            output_tokens: total
                .output_tokens
                .saturating_sub(self.billed.output_tokens),
            total_tokens: total.total_tokens.saturating_sub(self.billed.total_tokens),
            cached_input_tokens: total
                .cached_input_tokens
                .saturating_sub(self.billed.cached_input_tokens),
            cache_creation_input_tokens: total
                .cache_creation_input_tokens
                .saturating_sub(self.billed.cache_creation_input_tokens),
            reasoning_tokens: total
                .reasoning_tokens
                .saturating_sub(self.billed.reasoning_tokens),
            ..Default::default()
        };
        self.billed = total;
        let raw = if std::mem::take(&mut self.changed) {
            json!({"tokenUsage":self.report,"contextMessageCount":self.measured_prefix})
        } else {
            Value::Null
        };
        StreamFinal::new("codex", delta)
            .with_finish_reason(reason)
            .with_raw(raw)
    }
}
fn breakdown(value: &Value) -> Usage {
    let count = |field: &str| value[field].as_u64().unwrap_or(0);
    Usage {
        input_tokens: count("inputTokens"),
        output_tokens: count("outputTokens"),
        total_tokens: count("totalTokens"),
        cached_input_tokens: count("cachedInputTokens"),
        cache_creation_input_tokens: count("cacheWriteInputTokens"),
        reasoning_tokens: count("reasoningOutputTokens"),
        ..Default::default()
    }
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
        assert!(!usage.finish(FinishReason::ToolCalls).usage.has_values());
        usage.continue_after_tool(5);
        usage.observe(&report(1000, 1020));
        let first = usage.finish(FinishReason::ToolCalls);
        assert_eq!(first.usage.total_tokens, 1020);
        assert_eq!(first.raw["contextMessageCount"], 4);
        usage.continue_after_tool(7);
        usage.observe(&report(1000, 1020)); // Duplicate notification must not consume the anchor.
        usage.observe(&report(1100, 2140));
        let second = usage.finish(FinishReason::ToolCalls);
        assert_eq!(second.usage.total_tokens, 1120);
        assert_eq!(second.raw["contextMessageCount"], 6);
        assert_eq!(second.raw["tokenUsage"]["last"]["totalTokens"], 1120);
        assert!(!usage.finish(FinishReason::ToolCalls).usage.has_values());
    }
    #[test]
    fn reports_before_tool_calls_and_final_reports_use_the_current_context() {
        let mut usage = NativeUsage::default();
        usage.observe(&report(1000, 1020));
        assert!(usage.finish(FinishReason::ToolCalls).raw["contextMessageCount"].is_null());
        usage.continue_after_tool(5);
        usage.observe(&report(1100, 2140));
        let final_response = usage.finish(FinishReason::Stop);
        assert_eq!(final_response.usage.total_tokens, 1120);
        assert!(final_response.raw["contextMessageCount"].is_null());
    }
}
