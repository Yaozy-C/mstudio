use serde_json::Value;

#[derive(Default)]
pub struct EditProgress {
    error: Option<String>,
    repeats: usize,
}
impl EditProgress {
    pub fn observe(&mut self, name: &str, result: &Value) {
        if !super::operation_tools::is_edit(name) {
            return;
        }
        if result["applied"] == true {
            self.error = None;
            self.repeats = 0;
            return;
        }
        if let Some(error) = result["error"].as_str() {
            self.repeats = if self.error.as_deref() == Some(error) {
                self.repeats + 1
            } else {
                1
            };
            self.error = Some(error.into());
        }
    }
    pub fn stalled(&self) -> Option<String> {
        (self.repeats >= 3).then(|| format!("连续三次修改遇到相同错误，已停止重复尝试；已完成操作保留。请核实冲突原因后继续：{}", self.error.as_deref().unwrap_or_default()))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    #[test]
    fn reads_do_not_hide_repeated_failures_and_applied_edits_reset_them() {
        let mut progress = EditProgress::default();
        for _ in 0..3 {
            progress.observe("mstudio_edit", &json!({"error":"revision conflict"}));
            progress.observe("mstudio_inspect", &json!({"revision":4}));
        }
        assert!(progress.stalled().is_some());
        progress.observe("mstudio_edit", &json!({"applied":true}));
        assert!(progress.stalled().is_none());
        for error in ["a", "b", "c"] {
            progress.observe("mstudio_edit", &json!({"error":error}));
        }
        assert!(progress.stalled().is_none());
    }
}
