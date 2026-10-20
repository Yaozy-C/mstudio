use serde::Serialize;
use tauri::ipc::Channel;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Progress {
    stage: &'static str,
    completed: Option<u64>,
    total: Option<u64>,
    fragments: bool,
}
#[derive(Clone, Default)]
pub struct Reporter(pub Option<Channel<Progress>>);
impl Reporter {
    pub fn stage(&self, stage: &'static str) {
        self.send(stage, None, None);
    }
    pub fn download(&self, completed: u64, total: Option<u64>, fragments: bool) {
        if let Some(channel) = &self.0 {
            let _ = channel.send(Progress {
                stage: "downloading",
                completed: Some(completed),
                total,
                fragments,
            });
        }
    }
    pub fn send(&self, stage: &'static str, completed: Option<u64>, total: Option<u64>) {
        if let Some(channel) = &self.0 {
            let _ = channel.send(Progress {
                stage,
                completed,
                total,
                fragments: false,
            });
        }
    }
}
