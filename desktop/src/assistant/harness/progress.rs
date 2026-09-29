use serde_json::Value;

#[derive(Default)]
pub struct EditProgress {
    error: Option<String>,
    repeats: usize,
    generation_tasks: std::collections::HashSet<String>,
}
impl EditProgress {
    pub fn observe(&mut self, name: &str, result: &Value) {
        if name != "mstudio_edit" {
            return;
        }
        if result["applied"] == true {
            self.error = None;
            self.repeats = 0;
            for task in result["generationTasks"].as_array().into_iter().flatten() {
                if let Some(id) = task["id"].as_str() {
                    self.generation_tasks.insert(id.to_owned());
                }
            }
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
    pub fn step_limit(&self) -> usize {
        // Larger batches earn extra planning steps only after durable task receipts.
        if self.generation_tasks.len() >= 12 {
            64
        } else {
            16
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
    fn large_batches_earn_more_steps_from_unique_durable_receipts() {
        let mut progress = EditProgress::default();
        assert_eq!(progress.step_limit(), 16);
        let receipt = json!({"applied":true,"generationTasks":(0..12).map(|i| json!({"id":i.to_string()})).collect::<Vec<_>>()});
        progress.observe("mstudio_inspect", &receipt);
        assert_eq!(progress.step_limit(), 16);
        progress.observe("mstudio_edit", &receipt);
        assert_eq!(progress.step_limit(), 64);
        progress.observe("mstudio_edit", &json!({"applied":true}));
        assert_eq!(progress.step_limit(), 64);
    }
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
