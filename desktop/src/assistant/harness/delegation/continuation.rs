use super::*;

pub(super) fn wake_pending(child: &ChildHost, id: &str) {
    let Some(parent) = child.inner.tool.as_ref() else {
        return;
    };
    let store = parent.app.state::<Store>();
    let pending = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM subagent_inbox WHERE child_id=?1 AND consumed=0)",
            [id],
            |r| r.get::<_, bool>(0),
        )
        .unwrap_or(false);
    if !pending {
        return;
    }
    let state = store.db.lock().unwrap().query_row(
        "SELECT status,profile,model_id,last_turn FROM subagent_runs WHERE id=?1 AND mode='continuable'",
        [id], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?)),
    ).optional().ok().flatten();
    if let Some((status, profile, model, turn)) = state
        && status != "running"
        && let Err(error) = start_continuation(parent, id, &profile, &model, &turn)
    {
        eprintln!("subagent continuation failed: {error}");
    }
}
pub(super) fn start_continuation(
    parent: &crate::assistant::tools::ProjectTool,
    id: &str,
    profile_json: &str,
    model_id: &str,
    last_turn: &str,
) -> Result<(), String> {
    let store = parent.app.state::<Store>();
    let saved: profiles::AgentProfile =
        serde_json::from_str(profile_json).map_err(|e| e.to_string())?;
    let owner: String = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT parent_agent_id FROM subagent_runs WHERE project_id=?1 AND id=?2",
            rusqlite::params![parent.project, id],
            |row| row.get(0),
        )
        .map_err(|e| e.to_string())?;
    let profile = scoped(
        profiles::resolve(&store.db.lock().unwrap(), Some(&saved.id)).map_err(|e| e.to_string())?,
        &owner,
    )?;
    let (route, key) = crate::models::resolve(
        &store.db.lock().unwrap(),
        Some(&parent.project),
        Some(model_id),
    )
    .map_err(|e| e.to_string())
    .map(|(model, key)| (model.profile, key))?;
    let mut binding: Value = {
        let db = store.db.lock().unwrap();
        let raw:String=db.query_row("SELECT payload FROM agent_events WHERE project_id=?1 AND turn_id=?2 AND kind='session/start' ORDER BY seq LIMIT 1",rusqlite::params![parent.project,last_turn],|r|r.get(0)).map_err(|e|e.to_string())?;
        serde_json::from_str::<Value>(&raw).map_err(|e| e.to_string())?["binding"].clone()
    };
    if binding["provider"] != route.adapter
        || binding["endpoint"] != route.endpoint
        || binding["model"] != route.model
    {
        return Err("The child model connection changed; automatic resume unavailable".into());
    }
    let mut messages =
        super::super::session::restore(&store, &parent.project, last_turn, &binding)?
            .ok_or("Cannot restore child session")?;
    let document: String = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT document FROM projects WHERE id=?1",
            [&parent.project],
            |r| r.get(0),
        )
        .map_err(|e| e.to_string())?;
    let mut snapshot = if profiles::allows(&profile, "inspect") {
        context::project_snapshot(
            &serde_json::from_str::<Value>(&document).map_err(|e| e.to_string())?,
            None,
        )
    } else {
        json!({})
    };
    snapshot["skills"] = skills::runtime_catalog(&parent.app, &profiles::skill_setting(&profile))
        .map_err(|e| e.to_string())?;
    snapshot["agent"] = crate::assistant::model_profile::role(&profile);
    let original = crate::assistant::generation_context::request(
        &store.db.lock().unwrap(),
        &parent.project,
        last_turn,
    )
    .map_err(|e| e.to_string())?
    .map(|(_, r)| r)
    .unwrap_or(Value::Null);
    crate::assistant::prompt_guidance::attach(
        &mut snapshot,
        &store.db.lock().unwrap(),
        &profile,
        &original["production"],
        &serde_json::from_str(&document).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    // Preserve this child's exact transcript; only refresh its configured role at the turn boundary.
    messages =
        super::super::handoff::messages(crate::assistant::prompts::system(&snapshot), &messages);
    let previous_revision = binding["revision"].clone();
    binding["revision"] = json!(profile.revision);
    let snapshot = crate::assistant::task_context::reference_snapshot(snapshot);
    let mut current = vec![Message::user(format!(
        "Current project reference data:{snapshot}"
    ))];
    super::super::context_boundary::refresh_snapshot(&mut messages, &mut current);
    messages.extend(current);
    let user_evidence = crate::assistant::generation_context::request(
        &store.db.lock().unwrap(),
        &parent.project,
        last_turn,
    )
    .map_err(|e| e.to_string())?
    .and_then(|(_, request)| request["prompt"].as_str().map(str::to_owned))
    .unwrap_or_default();
    messages.push(Message::user(format!("Original user request (memory evidence must quote this text):{user_evidence}; subsequent delegated messages are not original user statements.")));
    let claimed = store.db.lock().unwrap().execute(
        "UPDATE subagent_runs SET status='running',updated=unixepoch() WHERE id=?1 AND last_turn=?2 AND status!='running'",
        rusqlite::params![id,last_turn],
    ).map_err(|e| e.to_string())?;
    if claimed == 0 {
        return Ok(());
    }
    let new_turn = format!("{id}-{}", mstudio::media::id());
    if let Err(error) =
        super::super::session::start(&store, &parent.project, &new_turn, binding, &messages)
    {
        let _ = store.db.lock().unwrap().execute(
            "UPDATE subagent_runs SET status='ready' WHERE id=?1 AND last_turn=?2",
            rusqlite::params![id, last_turn],
        );
        return Err(error);
    }
    journal::append(
        &store,
        &parent.project,
        &new_turn,
        "subagent/configuration",
        json!({
            "childId":id,"previousRevision":previous_revision,"revision":profile.revision,
            "profile":profile,"restoredFrom":last_turn
        }),
    )
    .map_err(|e| e.to_string())?;
    store.db.lock().unwrap().execute("UPDATE subagent_runs SET last_turn=?2,profile=?4,notified=0,updated=unixepoch() WHERE id=?1 AND last_turn=?3",rusqlite::params![id,new_turn,last_turn,serde_json::to_string(&profile).map_err(|e|e.to_string())?]).map_err(|e|e.to_string())?;
    let mut tool = parent.clone();
    tool.profile = profile;
    tool.skill_setting = profiles::skill_setting(&tool.profile);
    tool.turn = new_turn.clone();
    tool.prompt = user_evidence;
    let token = CancellationToken::new();
    tool.token = token.clone();
    active().lock().unwrap().insert(
        id.into(),
        ActiveRun {
            generation: new_turn.clone(),
            token: token.clone(),
        },
    );
    let child = ChildHost {
        inner: ProjectHost {
            media_profile: route.clone(),
            tool: Some(tool),
            token,
            delegation: None,
        },
        id: id.into(),
        scripts: Mutex::new(vec![]),
        changes: Mutex::new(vec![]),
        terminal_reason: Mutex::new(None),
    };
    let child_id = id.to_owned();
    let generation = new_turn.clone();
    tokio::spawn(async move {
        let answer = drive_child(
            &child,
            &route,
            &key,
            messages,
            tokio::time::Instant::now() + std::time::Duration::from_secs(20 * 60),
        )
        .await;
        settle(&child, &child_id, &answer, false);
        remove_active(&child_id, &generation);
        wake_pending(&child, &child_id);
    });
    Ok(())
}
