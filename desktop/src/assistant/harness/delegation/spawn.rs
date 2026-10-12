use super::*;

pub async fn execute(host: &ProjectHost, call: &ToolCall) -> Value {
    run(host, call).await.unwrap_or_else(
        |error| json!({"ok":false,"stopReason":"error","error":crate::assistant::model_feedback::error(&error),"code":"DELEGATION_FAILED"}),
    )
}

async fn run(host: &ProjectHost, call: &ToolCall) -> Result<Value, String> {
    let parent = host.tool.as_ref().ok_or("Delegation unavailable")?;
    if !profiles::allows(&parent.profile, "delegate") {
        return Err("Delegation permission required".into());
    }
    let route = host
        .delegation
        .as_ref()
        .ok_or("No model configured for this turn")?;
    let args = &serde_json::Value::Object(call.function.arguments.clone());
    let id = args["agentId"]
        .as_str()
        .ok_or("Missing specialist agentId")?;
    let task = args["task"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .ok_or("Missing explicit task")?;
    let provider_name = args["provider"].as_str().unwrap_or("spawn");
    if !["spawn", "fork"].contains(&provider_name) {
        return Err("Subagent provider not found".into());
    }
    let mode = args["mode"].as_str().unwrap_or("oneShot");
    if !["oneShot", "continuable"].contains(&mode) {
        return Err("Invalid subagent mode".into());
    }
    let background = args["runInBackground"]
        .as_bool()
        .unwrap_or(mode == "continuable");
    if id == parent.profile.id || id == "coordinator" {
        return Err("Cannot delegate to the coordinator or self".into());
    }
    let store = parent.app.state::<Store>();
    let profile =
        profiles::resolve(&store.db.lock().unwrap(), Some(id)).map_err(|e| e.to_string())?;
    let profile = scoped(profile, &parent.profile.id)?;
    if profile.id == "video-analyst" && provider_name == "fork" {
        return Err(
            "Video analysis needs independent context; use spawn with the original video".into(),
        );
    }

    let mut tool = parent.clone();
    tool.profile = profile;
    tool.skill_setting = profiles::skill_setting(&tool.profile);
    let raw: String = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT document FROM projects WHERE id=?1",
            [&parent.project],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let doc: Value = serde_json::from_str(&raw).map_err(|e| e.to_string())?;
    let mut snapshot = if profiles::allows(&tool.profile, "inspect") {
        context::project_snapshot(&doc, None)
    } else {
        json!({})
    };
    snapshot["skills"] =
        skills::runtime_catalog(&parent.app, &tool.skill_setting).map_err(|e| e.to_string())?;
    snapshot["agent"] = crate::assistant::model_profile::role(&tool.profile);
    let original = crate::assistant::generation_context::request(
        &store.db.lock().unwrap(),
        &parent.project,
        &parent.turn,
    )
    .map_err(|e| e.to_string())?
    .map(|(_, request)| request)
    .unwrap_or(Value::Null);
    crate::assistant::prompt_guidance::attach(
        &mut snapshot,
        &store.db.lock().unwrap(),
        &tool.profile,
        &original["production"],
        &doc,
    )
    .map_err(|e| e.to_string())?;
    let system = crate::assistant::prompts::system(&snapshot);
    let (child_profile, child_key, model_id) = if let Some(id) = args["modelId"].as_str() {
        let catalog = crate::models::catalog(&store.db.lock().unwrap(), Some(&parent.project))
            .map_err(|e| e.to_string())?;
        if !catalog.profiles.iter().any(|model| model.id == id) {
            return Err("Delegation needs a chat model connection ID, not an image/video model. Omit modelId to inherit the current chat model; generation uses the user-selected media model.".into());
        }
        let (model, key) =
            crate::models::resolve(&store.db.lock().unwrap(), Some(&parent.project), Some(id))
                .map_err(|e| e.to_string())?;
        (model.profile, key, model.id)
    } else {
        (
            route.profile.clone(),
            route.key.clone(),
            route.model_id.clone().unwrap_or_default(),
        )
    };
    if mode == "continuable" && model_id.is_empty() {
        return Err("Continuable children require a saved model connection".into());
    }
    if provider_name == "fork" && args["modelId"].as_str().is_some() {
        return Err("fork must retain the parent model; use spawn to change models".into());
    }
    let mut messages = super::super::handoff::messages(
        system,
        if provider_name == "fork" {
            &route.fork_history
        } else {
            &[]
        },
    );
    let sources = super::super::handoff_artifacts::resolve(
        &store,
        &parent.project,
        &doc,
        args.get("artifacts"),
    )?;
    if !sources.manifest.as_array().unwrap().is_empty()
        && !profiles::allows(&tool.profile, "inspect")
    {
        return Err("This specialist cannot consume project artifacts".into());
    }
    let user_refs = super::super::handoff_artifacts::user_refs(&original);
    let mut refs = user_refs.clone();
    refs.extend(sources.attachments.clone());
    let selection = json!({"models":original["production"]["models"],"parameters":original["production"]["task"]["parameters"]});
    let reference = crate::assistant::task_context::delegated_snapshot(
        snapshot,
        &doc,
        &tool.profile.id,
        &parent.turn,
        &refs,
    );
    let mut reference = reference;
    if tool.profile.id == "video-analyst" {
        crate::assistant::video_analysis::validate_video(&store, &refs, &child_profile)
            .map_err(|e| e.to_string())?;
        crate::assistant::video_analysis::isolate(&mut reference);
    }
    let mut reference = vec![super::super::context_source::snapshot(reference)];
    super::super::context_boundary::refresh_snapshot(&mut messages, &mut reference);
    messages.extend(reference);
    let mut instruction = format!(
        "Original user request (provided directly by the host):{}",
        original["prompt"].as_str().unwrap_or(&parent.prompt)
    );
    if tool.profile.id != "video-analyst" {
        instruction.push_str(&format!("\nUser-selected production settings for this turn (preserve, including the model):{selection}"));
    }
    let payload = crate::assistant::attachments::payload(
        &store,
        &doc,
        &instruction,
        &user_refs,
        &child_profile,
    )
    .map_err(|e| e.to_string())?;
    messages.push(crate::assistant::agent::convert(
        &json!({"role":"user","content":payload}),
    ));
    let instruction = super::super::handoff_artifacts::source_message(
        task,
        &parent.profile.id,
        &parent.turn,
        &sources.manifest,
    );
    let payload = crate::assistant::attachments::payload(
        &store,
        &doc,
        &instruction,
        &sources.attachments,
        &child_profile,
    )
    .map_err(|e| e.to_string())?;
    messages.push(crate::assistant::agent::convert(
        &json!({"role":"user","content":payload}),
    ));
    let task_message_index = messages.len() - 1;
    let child_id = format!("subagent-{}", mstudio::media::id());
    let child_turn = format!("{child_id}-1");
    let binding = json!({"provider":child_profile.adapter,"endpoint":child_profile.endpoint,"model":child_profile.model,"agentId":tool.profile.id,"revision":tool.profile.revision});
    {
        let db = store.db.lock().unwrap();
        let count:i64 = db.query_row("SELECT count(*) FROM subagent_runs WHERE project_id=?1 AND parent_agent_id=?2 AND mode='continuable' AND status='running'", rusqlite::params![parent.project,parent.profile.id], |r|r.get(0)).map_err(|e|e.to_string())?;
        if mode == "continuable" && count >= 8 {
            return Err("At most 8 continuable subagents can run simultaneously".into());
        }
        db.execute("INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn) VALUES(?1,?2,?3,?4,?5,?6,'running',?7,?8,?9)",rusqlite::params![child_id,parent.project,parent.turn,parent.profile.id,id,mode,serde_json::to_string(&tool.profile).map_err(|e|e.to_string())?,model_id,child_turn]).map_err(|e|e.to_string())?;
    }
    journal::append(&store, &parent.project, &child_turn, "subagent/created", json!({"childId":child_id,"parentTurn":parent.turn,"agentId":id,"provider":provider_name,"mode":mode})).map_err(|e|e.to_string())?;
    super::super::session::start(&store, &parent.project, &child_turn, binding, &messages)?;
    journal::append(&store, &parent.project, &child_turn, "subagent/context", json!({
        "childId":child_id,"parentTurn":parent.turn,"provider":provider_name,
        "inheritedMessageCount":if provider_name=="fork" { route.fork_history.len() } else { 0 },
        "taskMessageIndex":task_message_index,
        "source":super::super::mailbox::agent_source(&parent.profile.id, &parent.turn),
        "profile":tool.profile,"artifacts":sources.manifest
    })).map_err(|e| e.to_string())?;
    crate::assistant::child_activity::start(
        &store.db.lock().unwrap(),
        &parent.project,
        &child_turn,
        &child_id,
        &parent.turn,
        call.id.wire().as_ref(),
        id,
    )
    .map_err(|e| e.to_string())?;
    tool.turn = child_turn.clone();
    // Keep the immutable user evidence; a delegated task is not a user quote.
    tool.prompt = original["prompt"].as_str().unwrap_or(&parent.prompt).into();
    let token = host.token.child_token();
    tool.token = token.clone();
    tool.deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(20 * 60);
    let deadline = tool.deadline;
    let child = ChildHost {
        inner: ProjectHost {
            loaded_tools: Default::default(),
            media_profile: child_profile.clone(),
            token: token.clone(),
            tool: Some(tool),
            delegation: None,
        },
        id: child_id.clone(),
        scripts: Mutex::new(vec![]),
        changes: Mutex::new(vec![]),
        terminal_reason: Mutex::new(None),
    };
    active().lock().unwrap().insert(
        child_id.clone(),
        ActiveRun {
            generation: child_turn.clone(),
            project: parent.project.clone(),
            token,
        },
    );
    let spawned_id = child_id.clone();
    let generation = child_turn.clone();
    let running = json!({"ok":true,"agentId":id,"childId":child_id,"mode":mode,
        "canContinue":mode == "continuable","status":"running",
        "outputRef":super::super::agent_artifacts::reference(&child_id,&child_turn)});
    let handle = tokio::spawn(async move {
        let answer = drive_child(&child, &child_profile, &child_key, messages, deadline).await;
        // Always publish a durable notice: the parent may stop waiting at any instant.
        settle(&child, &spawned_id, &answer, false);
        remove_active(&spawned_id, &generation);
        wake_pending(&child, &spawned_id);
        super::execution::result(&child, &spawned_id, &generation, answer)
    });
    if background {
        return Ok(running);
    }
    match super::waiting::until(handle, route.deadline).await {
        Some(Ok(mut result)) => {
            result["mode"] = json!(mode);
            result["canContinue"] = json!(mode == "continuable");
            store
                .db
                .lock()
                .unwrap()
                .execute(
                    "UPDATE subagent_notices SET delivered=1 WHERE child_id=?1 AND delivered=0 AND json_extract(output,'$.outputRef.turnId')=?2",
                    rusqlite::params![child_id,child_turn],
                )
                .map_err(|e| e.to_string())?;
            Ok(result)
        }
        Some(Err(error)) => Err(error.to_string()),
        None => {
            let mut result = running;
            result["waitEnded"] = json!("parent_deadline");
            result["message"] = json!(
                "Child continues with its own time budget; await its completion notice. Do not redelegate this running work."
            );
            Ok(result)
        }
    }
}
