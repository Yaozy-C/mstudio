use super::{context, history, journal};
use crate::{database::Store, jobs};
use serde_json::json;
#[test]
fn context_uses_bounded_pairs_and_offloads_old_images() {
    let image = json!([{"type":"text","text":"old prompt"},{"type":"image_url","image_url":{"url":"data:verylargeimage"}}]);
    let mut previous = vec![];
    for _ in 0..30 {
        previous.push(history::Message {
            id: 0,
            role: "user".into(),
            content: "old".into(),
            model: "test".into(),
            attribution: None,
            payload: image.clone(),
        });
        previous.push(history::Message {
            id: 0,
            role: "assistant".into(),
            content: "x".repeat(2000),
            model: "test".into(),
            attribution: None,
            payload: json!("x".repeat(4000)),
        });
    }
    let messages = context::assemble(
        &previous,
        json!("new"),
        json!({"requirements":"locked fact"}),
    )
    .unwrap();
    assert!(!messages.to_string().contains("verylargeimage"));
    assert!(messages.to_string().contains("locked fact"));
    assert!(messages.as_array().unwrap().len() < 63);
    let large = "a".repeat(100_000);
    let input = context::assemble(&[], json!(large), json!({})).unwrap();
    assert_eq!(input.as_array().unwrap().last().unwrap()["content"], large);
}
#[test]
fn current_images_fit_by_compacting_optional_project_context() {
    let payload = json!([
        {"type":"text","text":"请参考这三张图继续制作"},
        {"type":"image_url","image_url":{"url":"data:first"}},
        {"type":"image_url","image_url":{"url":"data:second"}},
        {"type":"image_url","image_url":{"url":"data:third"}}
    ]);
    let snapshot = json!({
        "requirements":"保留商品原貌",
        "agent":{"name":"项目统筹","instructions":"按要求执行","canEdit":true},
        "skills":[{"name":"制作"}],
        "relevantNodes":[{"id":"shot","title":"当前镜头","kind":"shot","text":"镜头细节".repeat(20000)}],
        "memory":{"enabled":true,"entries":[{"content":"项目记忆".repeat(500)}]},
        "nodes":[{"id":"shot","title":"当前镜头"}]
    });
    let messages = context::assemble(&[], payload, snapshot).unwrap();
    let text = messages.to_string();
    assert!(text.contains("data:first"));
    assert!(text.contains("data:third"));
    assert!(text.contains("保留商品原貌"));
    assert!(!text.contains("镜头细节"));
}
#[test]
fn journal_preserves_failed_attempts_and_submission_reservations_are_idempotent() {
    let dir = std::env::temp_dir().join(format!("mstudio-journal-{}", mstudio::media::id()));
    let store = Store::open(dir.clone()).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','test','{}',0)", [])
        .unwrap();
    journal::append(&store, "p", "turn", "turn/start", json!({})).unwrap();
    journal::append(&store, "p", "turn", "turn/end", json!({"status":"failed"})).unwrap();
    assert_eq!(journal::page(&store, "p", None).unwrap().len(), 1);
    assert!(journal::page(&store, "other", None).unwrap().is_empty());
    let job = json!({"id":"run","projectId":"p","input":{"prompt":"test"},"endpoint":"fal-ai/test","status":"SUBMITTING","providerId":"fal"});
    assert!(jobs::reserve(&store, &job).unwrap());
    assert!(!jobs::reserve(&store, &job).unwrap());
    let mut changed = job.clone();
    changed["input"] = json!({"prompt":"changed"});
    assert!(jobs::reserve(&store, &changed).is_err());
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}
#[test]
fn snapshot_keeps_requirements_and_references() {
    let document = json!({"name":"test","revision":9,"nodes":[{"id":"shot","title":"镜头一","text":"动作","references":[{"assetId":"ref","purpose":"只参考推进动作","start":2,"end":5}],"resultAssetId":"chosen"}],"creation":{"intent":"整片主题","essential":"饮料跳进包","preserve":"网袋不能变","stage":"editing","feedback":[{"id":"done","resolved":true,"text":"已确认"},{"id":"todo","clipId":"clip","assetId":"old-version","time":2,"sourceTime":4,"revision":8,"text":"只修网袋","preserve":"其他部分不变"}]}});
    let snapshot = context::project_snapshot(&document, Some("shot"));
    assert_eq!(snapshot["creation"]["essential"], "饮料跳进包");
    assert!(snapshot["creation"].get("unresolvedFeedback").is_none());
    assert!(!snapshot.to_string().contains("只修网袋"));
    assert_eq!(snapshot["relevantNodes"][0]["resultAssetId"], "chosen");
    assert_eq!(
        snapshot["relevantNodes"][0]["references"][0]["purpose"],
        "只参考推进动作"
    );
    assert!(context::assemble(&[], json!("这里怎么修"), snapshot).is_ok());
}
#[test]
fn creative_plan_and_linked_shot_context_are_bounded_and_readable() {
    let document = json!({"nodes":[
        {"id":"p","kind":"screenplay","screenplay":{"story":"begin then end","sound":"natural"}},
        {"id":"s","kind":"shot","text":"action","shot":{"screenplayId":"p","order":1,"duration":6,"dialogue":"Let's pack"}}
    ]});
    let screenplay = super::creative_context::node_context(&document, &document["nodes"][0]);
    assert_eq!(screenplay["shots"][0]["id"], "s");
    let shot = context::project_snapshot(&document, Some("s"));
    assert_eq!(
        shot["relevantNodes"][0]["creative"]["dialogue"],
        "Let's pack"
    );
    let schema = super::tool_schema::schema();
    assert!(
        schema["properties"]["operations"]["items"]["oneOf"][0]["properties"]["kind"]["enum"]
            .as_array()
            .unwrap()
            .contains(&json!("screenplay"))
    );
}

#[test]
fn read_only_agents_do_not_receive_edit_tools_or_creative_persona() {
    let mut profile = super::profiles::defaults("");
    profile.tool_ids = vec!["project-read".into()];
    let schema = super::tool_schema::for_profile(&profile);
    assert!(
        !schema["properties"]["action"]["enum"]
            .as_array()
            .unwrap()
            .contains(&json!("edit"))
    );
    assert!(schema["properties"].get("operations").is_none());
    let input = context::assemble(
        &[],
        json!("分析"),
        json!({"agent":{"name":"审片员","instructions":"只分析节奏","canEdit":false}}),
    )
    .unwrap();
    let system = input[0]["content"].as_str().unwrap();
    assert!(system.contains("只分析节奏"));
    assert!(!system.contains("add_node"));
    // High local estimates must not reject current role instructions before a provider call.
    assert!(
        context::assemble(
            &[],
            json!("hi"),
            json!({"agent":{"instructions":"x".repeat(100000)}})
        )
        .is_ok()
    );
}
#[test]
fn interrupted_tools_are_recalled_only_for_same_project_role_and_task() {
    let dir = std::env::temp_dir().join(format!("mstudio-recovery-{}", mstudio::media::id()));
    let store = Store::open(dir.clone()).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','test','{}',0)", [])
        .unwrap();
    journal::append(
        &store,
        "p",
        "turn",
        "user/message",
        json!({"text":"create shot"}),
    )
    .unwrap();
    journal::append(
        &store,
        "p",
        "turn",
        "tool/result",
        json!({"callId":"edit-1","name":"mstudio_edit","result":{"applied":true,"changed":[{"id":"shot-created"}]}}),
    )
    .unwrap();
    use super::task_context::{self, Scope};
    let mut scope = Scope {
        task_id: "task".into(),
        agent_id: "coordinator".into(),
        targets: vec![],
        view: "project".into(),
        original_instruction: "create shot".into(),
    };
    history::append_attributed(&store,"p","create shot",&json!("create shot"),"stopped","m",Some(&json!({"agentId":"coordinator","turnId":"turn","status":"cancelled","taskScope":scope}))).unwrap();
    let recovery = task_context::recovery(&store, "p", &scope, "turn").unwrap();
    assert!(recovery.to_string().contains("shot-created"));
    assert!(
        task_context::recovery(&store, "p", &scope, "different-turn")
            .unwrap()
            .is_null()
    );
    assert!(
        task_context::recovery(&store, "other", &scope, "turn")
            .unwrap()
            .is_null()
    );
    scope.agent_id = "color".into();
    assert!(
        task_context::recovery(&store, "p", &scope, "turn")
            .unwrap()
            .is_null()
    );
    scope.agent_id = "coordinator".into();
    scope.task_id = "new-task".into();
    assert!(
        task_context::recovery(&store, "p", &scope, "turn")
            .unwrap()
            .is_null()
    );
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn attribution_survives_reopen_and_activity_is_scoped_to_project_and_turn() {
    let dir = std::env::temp_dir().join(format!("mstudio-attribution-{}", mstudio::media::id()));
    {
        let store = Store::open(dir.clone()).unwrap();
        store
            .db
            .lock()
            .unwrap()
            .execute("INSERT INTO projects VALUES('p','test','{}',0)", [])
            .unwrap();
        let meta = json!({"agentId":"reviewer","agentName":"审片员","turnId":"turn-a","modelName":"Review"});
        history::append_attributed(
            &store,
            "p",
            "check",
            &json!("check"),
            "done",
            "review",
            Some(&meta),
        )
        .unwrap();
        journal::append(&store, "p", "turn-a", "tool/result", json!({"owner":"a"})).unwrap();
        journal::append(&store, "p", "turn-b", "tool/result", json!({"owner":"b"})).unwrap();
    }
    let store = Store::open(dir.clone()).unwrap();
    let messages = history::read(&store, "p").unwrap();
    assert_eq!(
        messages[0].attribution.as_ref().unwrap()["agentId"],
        "reviewer"
    );
    assert_eq!(
        messages[1].attribution.as_ref().unwrap()["turnId"],
        "turn-a"
    );
    let events = journal::turn_page(&store, "p", "turn-a", None).unwrap();
    assert_eq!(events.len(), 1);
    assert_eq!(events[0]["payload"]["owner"], "a");
    assert!(
        journal::turn_page(&store, "other", "turn-a", None)
            .unwrap()
            .is_empty()
    );
    drop(store);
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn role_catalog_has_one_source_and_snapshots_do_not_duplicate_role_rules() {
    let snapshot = json!({"agent":{"name":"总 Agent","tools":["agent-delegate"]},"specialists":[{"id":"specialist-unique"}],"skills":[]});
    let messages = context::assemble(&[], json!("修改时长"), snapshot.clone()).unwrap();
    let all = messages.to_string();
    assert_eq!(all.matches("specialist-unique").count(), 1);
    let child = super::task_context::reference_snapshot(snapshot);
    assert!(child.get("specialists").is_none());
    assert!(child.get("agent").is_none());
}
