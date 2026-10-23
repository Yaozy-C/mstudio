use super::*;

pub(super) fn wake_pending(child: &ChildHost, id: &str) -> Result<(), String> {
    let parent = child
        .inner
        .tool
        .as_ref()
        .ok_or("Subagent project context missing")?;
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
        .map_err(|error| error.to_string())?;
    if !pending {
        return Ok(());
    }
    let state = store.db.lock().unwrap().query_row(
        "SELECT status,profile,model_id,last_turn FROM subagent_runs WHERE id=?1 AND mode='continuable'",
        [id], |r| Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,String>(3)?)),
    ).map_err(|error| error.to_string())?;
    let (status, profile, model, turn) = state;
    if status != "running" {
        start_continuation(parent, id, &profile, &model, &turn)?;
    }
    Ok(())
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
    let (owner, parent_turn): (String, String) = store
        .db
        .lock()
        .unwrap()
        .query_row(
            "SELECT parent_agent_id,parent_turn FROM subagent_runs WHERE project_id=?1 AND id=?2",
            rusqlite::params![parent.project, id],
            |row| Ok((row.get(0)?, row.get(1)?)),
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
    let mut tool = parent.clone();
    tool.profile = profile;
    tool.skill_setting = profiles::skill_setting(&tool.profile);
    tool.turn = last_turn.into();
    crate::assistant::turn_context::refresh(&mut tool, &mut messages)?;
    let previous_revision = binding["revision"].clone();
    binding["revision"] = json!(tool.profile.revision);
    let original_source = crate::assistant::generation_context::request(
        &store.db.lock().unwrap(),
        &parent.project,
        last_turn,
    )
    .map_err(|e| e.to_string())?;
    let request_turn = original_source.as_ref().map(|(turn, _)| turn.clone());
    tool.prompt = original_source
        .and_then(|(_, request)| {
            request["production"]["instruction"]
                .as_str()
                .map(str::to_owned)
        })
        .unwrap_or_default();
    let new_turn = format!("{id}-{}", mstudio::media::id());
    let started = persistence::resume(&mut store.db.lock().unwrap(), persistence::Start {
        child: id, project: &parent.project, parent_turn: &parent.turn,
        parent_agent: &owner, turn: &new_turn, profile: &tool.profile,
        binding, messages: &messages, events: vec![
            ("request/source", json!({"requestTurnId":request_turn})),
            ("subagent/configuration", json!({
                "childId":id,"previousRevision":previous_revision,"revision":tool.profile.revision,
                "profile":tool.profile,"restoredFrom":last_turn,"requestTurnId":request_turn
            })),
        ],
    }, last_turn).map_err(|error| error.to_string())?;
    if !started {
        return Ok(());
    }
    tool.turn = new_turn.clone();
    let token = CancellationToken::new();
    tool.token = token.clone();
    tool.deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(20 * 60);
    let deadline = tool.deadline;
    active().lock().unwrap().insert(
        id.into(),
        ActiveRun {
            generation: new_turn.clone(),
            project: tool.project.clone(),
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
        parent_turn,
        changes: Mutex::new(vec![]),
        terminal_reason: Mutex::new(None),
    };
    let child_id = id.to_owned();
    let generation = new_turn.clone();
    tokio::spawn(async move {
        let answer = drive_child(&child, &route, &key, messages, deadline).await;
        execution::finish(&child, &child_id, &generation, &answer);
    });
    Ok(())
}
