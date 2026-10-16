use super::{
    super::{Host, ProjectHost, delegation, run, session::Session},
    support::*,
};
use rig_core::{
    completion::ToolDefinition,
    message::{AssistantContent, Message, ToolCall},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};
use tokio::sync::{Semaphore, mpsc, oneshot};

struct BackgroundChildren {
    inner: Arc<TestHost>,
    policy: ProjectHost,
    gates: Mutex<HashMap<String, oneshot::Receiver<()>>>,
    pool: Arc<Semaphore>,
    outstanding: Arc<AtomicUsize>,
    started: mpsc::UnboundedSender<String>,
    finished: mpsc::UnboundedSender<String>,
}
impl Host for BackgroundChildren {
    fn pending_children(&self) -> bool {
        self.outstanding.load(Ordering::SeqCst) > 0
    }
    fn token(&self) -> &tokio_util::sync::CancellationToken {
        &self.inner.token
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        self.inner.record(kind, value)
    }
    fn definitions(&self) -> Vec<ToolDefinition> {
        let mut tools = self.inner.definitions();
        tools.push(delegation::definition());
        tools
    }
    fn parallel_safe(&self, call: &ToolCall) -> bool {
        self.policy.parallel_safe(call)
    }
    fn injected(&self) -> Result<Vec<Message>, String> {
        self.inner.injected()
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        if call.function.name != "mstudio_delegate" {
            self.inner
                .trace
                .lock()
                .unwrap()
                .push("independent work".into());
            assert!(self.pending_children());
            return json!({"ok":true});
        }
        let id = call.id.wire().to_string();
        let gate = self.gates.lock().unwrap().remove(&id).unwrap();
        let inner = self.inner.clone();
        let outstanding = self.outstanding.clone();
        let pool = self.pool.clone();
        let started = self.started.clone();
        let finished = self.finished.clone();
        let child_id = id.clone();
        self.outstanding.fetch_add(1, Ordering::SeqCst);
        tokio::spawn(async move {
            let _permit = pool.acquire().await.unwrap();
            let active = inner.active.fetch_add(1, Ordering::SeqCst) + 1;
            inner.peak.fetch_max(active, Ordering::SeqCst);
            started.send(child_id.clone()).unwrap();
            gate.await.unwrap();
            let failed = child_id == "1";
            let notice = json!({"outputRef":{"id":child_id},"ok":!failed,
                "stopReason":if failed {"error"} else {"completed"}});
            inner.inbox.lock().unwrap().push(Message::user(format!(
                "Subagent {child_id} settled this turn with status {}. Final message: {notice}",
                if failed { "failed" } else { "completed" }
            )));
            inner.active.fetch_sub(1, Ordering::SeqCst);
            outstanding.fetch_sub(1, Ordering::SeqCst);
            finished.send(child_id).unwrap();
        });
        json!({"ok":true,"childId":id,"status":"running","outputRef":{"id":id}})
    }
}

#[tokio::test]
async fn parent_continues_and_parks_before_six_children_finish_then_collects_notices() {
    tokio::time::timeout(std::time::Duration::from_secs(5), background_roundtrip())
        .await
        .expect("parent waited for child completion or lost a completion notice");
}

async fn background_roundtrip() {
    let (started, mut starts) = mpsc::unbounded_channel();
    let (finished, mut finishes) = mpsc::unbounded_channel();
    let mut gates = HashMap::new();
    let mut releases = HashMap::new();
    for i in 0..6 {
        let (send, receive) = oneshot::channel();
        gates.insert(i.to_string(), receive);
        releases.insert(i.to_string(), send);
    }
    let host = BackgroundChildren {
        inner: Arc::new(TestHost::default()),
        policy: ProjectHost {
            tool: None,
            media_profile: profile(),
            token: Default::default(),
            delegation: None,
        },
        gates: Mutex::new(gates),
        pool: Arc::new(Semaphore::new(super::super::MAX_CONCURRENCY)),
        outstanding: Arc::new(AtomicUsize::new(0)),
        started,
        finished,
    };
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Ok(reply(
            (0..6)
                .map(|i| {
                    AssistantContent::ToolCall(call(
                        &i.to_string(),
                        "mstudio_delegate",
                        json!({"agentId":"image",
                "task":format!("Create shot {i}"),"expectedGenerations":[],
                "mode":if i % 2 == 0 {"oneShot"} else {"continuable"}}),
                    ))
                })
                .collect(),
        )),
        Ok(reply(vec![AssistantContent::ToolCall(call(
            "read",
            "read",
            json!({}),
        ))])),
        Ok(reply(vec![AssistantContent::text(
            "独立工作完成，等待子任务通知。",
        )])),
    ]);
    let answer = run(
        &model,
        &profile(),
        &host,
        Session::new(vec![Message::user(
            "Create six shots and do independent work",
        )]),
        false,
        "",
    )
    .await
    .unwrap();
    assert!(answer.contains("等待子任务通知"));
    assert!(host.pending_children());
    assert_eq!(*host.inner.trace.lock().unwrap(), ["independent work"]);
    assert_eq!(model.requests.lock().unwrap().len(), 3);
    let mut first = Vec::new();
    for _ in 0..4 {
        first.push(starts.recv().await.unwrap());
    }
    assert!(starts.try_recv().is_err());
    releases.remove(&first[1]).unwrap().send(()).unwrap();
    assert_eq!(finishes.recv().await.unwrap(), first[1]);
    let fifth = starts.recv().await.unwrap();
    releases.remove(&fifth).unwrap().send(()).unwrap();
    assert_eq!(finishes.recv().await.unwrap(), fifth);
    starts.recv().await.unwrap();
    for (_, release) in releases {
        release.send(()).unwrap();
    }
    for _ in 0..4 {
        finishes.recv().await.unwrap();
    }
    assert!(!host.pending_children());
    assert_eq!(host.inner.peak.load(Ordering::SeqCst), 4);
    assert_eq!(host.inner.active.load(Ordering::SeqCst), 0);
    let mut history = model
        .requests
        .lock()
        .unwrap()
        .last()
        .unwrap()
        .chat_history
        .clone();
    history.push(Message::assistant(answer));
    let followup = TestModel::default();
    followup
        .responses
        .lock()
        .unwrap()
        .push_back(Ok(reply(vec![AssistantContent::text("已整合结果。")])));
    let answer = run(
        &followup,
        &profile(),
        &host,
        Session::new(history),
        false,
        "",
    )
    .await
    .unwrap();
    assert!(answer.contains("已整合结果"));
    assert!(answer.contains("子任务 1 执行异常"));
    let requests = followup.requests.lock().unwrap();
    assert_eq!(requests.len(), 1);
    let notices = requests[0]
        .chat_history
        .iter()
        .filter(|message| {
            serde_json::to_string(message)
                .unwrap()
                .contains("settled this turn with status")
        })
        .count();
    assert_eq!(notices, 6);
    assert!(host.inner.inbox.lock().unwrap().is_empty());
}
