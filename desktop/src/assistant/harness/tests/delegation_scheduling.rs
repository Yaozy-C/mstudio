use super::{
    super::{Host, ProjectHost, delegation, run, scheduler, session::Session},
    support::*,
};
use rig_core::{
    completion::ToolDefinition,
    message::{AssistantContent, Message, ToolCall},
};
use serde_json::{Value, json};
use std::{
    collections::HashMap,
    sync::{Mutex, atomic::Ordering},
};
use tokio::sync::{mpsc, oneshot};

struct WaitingChildren {
    inner: TestHost,
    policy: ProjectHost,
    gates: Mutex<HashMap<String, oneshot::Receiver<()>>>,
    started: mpsc::UnboundedSender<String>,
    failing: Option<String>,
}
impl Host for WaitingChildren {
    fn token(&self) -> &tokio_util::sync::CancellationToken {
        &self.inner.token
    }
    fn record(&self, kind: &str, value: Value) -> Result<(), String> {
        self.inner.record(kind, value)
    }
    fn definitions(&self) -> Vec<ToolDefinition> {
        vec![delegation::definition()]
    }
    fn parallel_safe(&self, call: &ToolCall) -> bool {
        self.policy.parallel_safe(call)
    }
    async fn execute(&self, call: &ToolCall) -> Value {
        let id = call.id.wire().to_string();
        let count = self.inner.active.fetch_add(1, Ordering::SeqCst) + 1;
        self.inner.peak.fetch_max(count, Ordering::SeqCst);
        let gate = self.gates.lock().unwrap().remove(&id).unwrap();
        self.started.send(id.clone()).unwrap();
        gate.await.unwrap();
        self.inner.active.fetch_sub(1, Ordering::SeqCst);
        if self.failing.as_ref() == Some(&id) {
            json!({"ok":false,"childId":id,"stopReason":"error","error":"child failed"})
        } else {
            json!({"ok":true,"childId":id,"stopReason":"completed","answer":id})
        }
    }
}

fn setup(
    count: usize,
) -> (
    WaitingChildren,
    mpsc::UnboundedReceiver<String>,
    HashMap<String, oneshot::Sender<()>>,
) {
    let (started, receiver) = mpsc::unbounded_channel();
    let mut gates = HashMap::new();
    let mut releases = HashMap::new();
    for i in 0..count {
        let (send, receive) = oneshot::channel();
        gates.insert(i.to_string(), receive);
        releases.insert(i.to_string(), send);
    }
    (
        WaitingChildren {
            inner: TestHost::default(),
            policy: ProjectHost {
                tool: None,
                media_profile: profile(),
                token: Default::default(),
                delegation: None,
            },
            gates: Mutex::new(gates),
            started,
            failing: None,
        },
        receiver,
        releases,
    )
}

fn delegate(i: usize) -> ToolCall {
    call(
        &i.to_string(),
        "mstudio_delegate",
        json!({
            "agentId":"image", "task":format!("Update shot {i}"),
            "mode":"continuable", "runInBackground":false, "expectedGenerations":[],
        }),
    )
}

#[tokio::test]
async fn six_foreground_children_overlap_and_parent_waits_for_every_result() {
    let (host, mut started, mut releases) = setup(6);
    let model = TestModel::default();
    model.responses.lock().unwrap().extend([
        Ok(reply(
            (0..6)
                .map(|i| AssistantContent::ToolCall(delegate(i)))
                .collect(),
        )),
        Ok(reply(vec![AssistantContent::text(
            "All children collected",
        )])),
    ]);
    let route = profile();
    let work = run(
        &model,
        &route,
        &host,
        Session::new(vec![Message::user("Update six shots")]),
        false,
        "",
    );
    let control = async {
        let mut first = Vec::new();
        for _ in 0..4 {
            first.push(started.recv().await.unwrap());
        }
        first.sort();
        assert_eq!(first, ["0", "1", "2", "3"]);
        assert!(started.try_recv().is_err());
        releases.remove("1").unwrap().send(()).unwrap();
        assert_eq!(started.recv().await.unwrap(), "4");
        releases.remove("4").unwrap().send(()).unwrap();
        assert_eq!(started.recv().await.unwrap(), "5");
        for id in ["2", "3", "5"] {
            releases.remove(id).unwrap().send(()).unwrap();
        }
        // The slow first child still owns a foreground wait: no next model step.
        assert_eq!(model.requests.lock().unwrap().len(), 1);
        releases.remove("0").unwrap().send(()).unwrap();
    };
    let (answer, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(work, control)
    })
    .await
    .expect("delegations ran sequentially or the parent failed to collect results");
    assert_eq!(answer.unwrap(), "All children collected");
    assert_eq!(host.inner.peak.load(Ordering::SeqCst), 4);
    assert_eq!(host.inner.active.load(Ordering::SeqCst), 0);
    let requests = model.requests.lock().unwrap();
    assert_eq!(requests.len(), 2);
    let results: Vec<_> = requests[1]
        .chat_history
        .iter()
        .flat_map(session_results)
        .collect();
    assert_eq!(results, (0..6).map(|i| i.to_string()).collect::<Vec<_>>());
}

fn session_results(message: &Message) -> Vec<String> {
    match message {
        Message::User { content } => content
            .iter()
            .filter_map(|part| match part {
                rig_core::message::UserContent::ToolResult(result) => {
                    Some(result.call.wire().to_string())
                }
                _ => None,
            })
            .collect(),
        _ => vec![],
    }
}

#[tokio::test]
async fn failed_child_does_not_abandon_other_foreground_results() {
    let (mut host, mut started, mut releases) = setup(3);
    host.failing = Some("1".into());
    let calls: Vec<_> = (0..3).map(delegate).collect();
    let mut session = Session::new(vec![]);
    let work = scheduler::execute(&host, &mut session, &calls);
    let control = async {
        for _ in 0..3 {
            started.recv().await.unwrap();
        }
        for id in ["1", "2", "0"] {
            releases.remove(id).unwrap().send(()).unwrap();
        }
    };
    let (result, ()) = tokio::time::timeout(std::time::Duration::from_secs(5), async {
        tokio::join!(work, control)
    })
    .await
    .unwrap();
    result.unwrap();
    let events = host.inner.events.lock().unwrap();
    let results: Vec<_> = events
        .iter()
        .filter(|(kind, _)| kind == "tool/result")
        .collect();
    assert_eq!(results.len(), 3);
    assert_eq!(results[0].1["result"]["ok"], true);
    assert_eq!(results[1].1["result"]["ok"], false);
    assert_eq!(results[2].1["result"]["ok"], true);
    assert_eq!(session.messages.len(), 3);
}
