mod asset_sources;
mod dependencies;
#[cfg(test)]
mod dependency_tests;
pub mod generation_wait;
pub(crate) mod observations;
pub mod production;
pub mod receipts;
pub mod runtime;
use crate::{
    assistant::{permissions, profiles::AgentProfile},
    database::Store,
};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use tauri::{Emitter, Manager};

pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch("CREATE TABLE IF NOT EXISTS project_executions(project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE, execution_id TEXT NOT NULL, arguments TEXT NOT NULL, receipt TEXT NOT NULL, PRIMARY KEY(project_id,execution_id));
        CREATE TABLE IF NOT EXISTS project_observations(project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE, turn_id TEXT NOT NULL,target TEXT NOT NULL,value TEXT NOT NULL, PRIMARY KEY(project_id,turn_id,target));")?;
    Ok(())
}
pub fn load(db: &Connection, id: &str) -> Result<Value> {
    let raw: String = db
        .query_row("SELECT document FROM projects WHERE id=?1", [id], |r| {
            r.get(0)
        })
        .context("Project no longer exists")?;
    Ok(serde_json::from_str(&raw)?)
}
pub fn persist(db: &Connection, before: &Value, after: &Value) -> Result<()> {
    let id = before["id"].as_str().context("Missing project ID")?;
    let mut after = if before["removedAssetIds"]
        .as_array()
        .is_some_and(|ids| !ids.is_empty())
    {
        runtime::execute(json!({"action":"prune_deleted","document":after,"current":before}))?["document"].clone()
    } else {
        after.clone()
    };
    after["storageVersion"] = json!(before["storageVersion"].as_u64().unwrap_or(0) + 1);
    let name = after["name"]
        .as_str()
        .filter(|s| !s.trim().is_empty())
        .context("Missing project name")?;
    ensure!(
        before["id"] == after["id"],
        "Cannot change project identity"
    );
    crate::project_storage::remember(db, id, before)?;
    ensure!(
        db.execute(
            "UPDATE projects SET name=?2,document=?3,updated=unixepoch() WHERE id=?1",
            params![id, name, after.to_string()]
        )? == 1,
        "Project no longer exists"
    );
    crate::project_storage::remember(db, id, &after)?;
    crate::project_storage::delete_media::commit(db, before, &after)?;
    Ok(())
}
pub fn execute(
    store: &Store,
    profile: &AgentProfile,
    project: &str,
    turn: &str,
    execution: &str,
    args: Value,
) -> Result<Value> {
    let schema = crate::assistant::tool_schema::for_profile(profile);
    let issues = crate::assistant::harness::schema::issues(&schema, &args);
    if !issues.is_empty() {
        return Ok(crate::assistant::harness::schema::rejection(issues));
    }
    let generation = crate::assistant::generation_context::production(store, project, turn)?;
    let models = crate::models::media::resolved(&*store.conn()?)?;
    let mut db = store.conn()?;
    let tx = db.transaction()?;
    let before = load(&tx, project)?;
    if args["action"] == "inspect" {
        let mut result =
            runtime::execute(json!({"action":"inspect","document":before,"args":args}))?;
        asset_sources::enrich(&tx, project, &args, &mut result)?;
        observations::remember(
            &tx,
            project,
            turn,
            &observations::read_targets(&result),
            &before,
        )?;
        tx.commit()?;
        return Ok(result);
    }
    ensure!(args["action"] == "edit", "Unknown project action");
    if let Some((original, receipt)) = tx.query_row("SELECT arguments,receipt FROM project_executions WHERE project_id=?1 AND execution_id=?2", params![project,execution], |r| Ok((r.get::<_, String>(0)?,r.get::<_, String>(1)?))).optional()? {
        ensure!(serde_json::from_str::<Value>(&original)? == args, "Execution identity reused with different arguments");
        return Ok(serde_json::from_str(&receipt)?);
    }
    permissions::validate(profile, &args, &before)?;
    let direct = observations::required(&before, &before, &args);
    if let Some(conflict) = observations::check(&tx, project, turn, &direct, &before)? {
        return Ok(conflict);
    }
    let context = (!generation.is_null()).then(
        || json!({"callId":execution,"turnId":generation["turnId"],"turn":generation["turn"]}),
    );
    let value = runtime::execute(
        json!({"action":"edit","document":before,"args":args,"context":context,"executionId":execution,"models":models}),
    )?;
    if value.get("error").is_some() {
        return Ok(value);
    }
    let after = &value["document"];
    ensure!(
        after.is_object() && value["receipt"]["applied"] == true,
        "Invalid domain edit output"
    );
    let required = observations::required(&before, after, &args);
    if let Some(conflict) = observations::check(&tx, project, turn, &required, &before)? {
        return Ok(conflict);
    }
    persist(&tx, &before, after)?;
    let old_targets = observations::targets(&before);
    let changed = observations::targets(after)
        .into_iter()
        .filter(|(key, value)| old_targets.get(key) != Some(value))
        .map(|(key, _)| key)
        .collect();
    observations::remember(&tx, project, turn, &changed, after)?;
    tx.execute(
        "INSERT INTO project_executions VALUES(?1,?2,?3,?4)",
        params![
            project,
            execution,
            args.to_string(),
            value["receipt"].to_string()
        ],
    )?;
    tx.commit()?;
    store.project_changed(project);
    Ok(value["receipt"].clone())
}
#[tauri::command]
pub fn get_project(store: tauri::State<Store>, project_id: String) -> Result<Value, String> {
    load(&*store.conn().map_err(|e| e.to_string())?, &project_id).map_err(|e| e.to_string())
}
pub fn notify(app: &tauri::AppHandle, project: &str) {
    let _ = app.emit("project-changed", json!({"projectId":project}));
}
pub async fn tool(
    app: tauri::AppHandle,
    profile: AgentProfile,
    project: String,
    turn: String,
    execution: String,
    args: Value,
    token: tokio_util::sync::CancellationToken,
) -> Value {
    let _files = app.state::<Store>().files.clone().read_owned().await;
    let action = args["action"].clone();
    let target = project.clone();
    let worker = app.clone();
    let result = tauri::async_runtime::spawn_blocking(move || {
        if token.is_cancelled() { return Ok(json!({"error":"Stopped before transaction", "code":"ABORTED_BEFORE_DISPATCH", "stage":"dispatch", "outcome":"not_executed"})); }
        execute(
            &worker.state::<Store>(),
            &profile,
            &project,
            &turn,
            &execution,
            args,
        )
    })
    .await;
    let value = match result {
        Ok(Ok(value)) => value,
        Ok(Err(error)) => {
            json!({"error":error.to_string(),"code":"PROJECT_REJECTED","stage":"transaction","outcome":"not_executed","recovery":{"action":"correct_request"}})
        }
        Err(error) => {
            json!({"error":error.to_string(),"code":"EXECUTION_UNKNOWN","stage":"execution","outcome":"unknown","recovery":{"action":"read_receipt"}})
        }
    };
    if action == "edit" && value["applied"] == true {
        notify(&app, &target);
    }
    value
}
#[cfg(test)]
mod tests;

#[cfg(test)]
mod production_tests;
