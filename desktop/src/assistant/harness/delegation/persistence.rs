use super::*;
use rusqlite::{Connection, params};

pub(super) struct Start<'a> {
    pub child: &'a str,
    pub project: &'a str,
    pub parent_turn: &'a str,
    pub parent_agent: &'a str,
    pub turn: &'a str,
    pub profile: &'a profiles::AgentProfile,
    pub binding: Value,
    pub messages: &'a [Message],
    pub events: Vec<(&'static str, Value)>,
}

impl Start<'_> {
    fn session(&self, db: &Connection) -> anyhow::Result<()> {
        super::super::session::start_in(
            db,
            self.project,
            self.turn,
            self.binding.clone(),
            self.messages,
        )?;
        for (kind, value) in &self.events {
            journal::insert(db, self.project, self.turn, kind, value.clone())?;
        }
        Ok(())
    }
}

pub(super) fn spawn(
    db: &mut Connection,
    start: Start<'_>,
    mode: &str,
    model: &str,
    call: &str,
) -> anyhow::Result<()> {
    let tx = db.transaction()?;
    tx.execute("INSERT INTO subagent_runs(id,project_id,parent_turn,parent_agent_id,agent_id,mode,status,profile,model_id,last_turn) VALUES(?1,?2,?3,?4,?5,?6,'running',?7,?8,?9)",params![start.child,start.project,start.parent_turn,start.parent_agent,start.profile.id,mode,serde_json::to_string(start.profile)?,model,start.turn])?;
    start.session(&tx)?;
    crate::assistant::child_activity::start(
        &tx,
        start.project,
        start.turn,
        start.child,
        start.parent_turn,
        call,
        &start.profile.id,
    )?;
    tx.commit()?;
    Ok(())
}

pub(super) fn resume(
    db: &mut Connection,
    start: Start<'_>,
    previous: &str,
) -> anyhow::Result<bool> {
    let tx = db.transaction()?;
    let changed = tx.execute("UPDATE subagent_runs SET status='running',last_turn=?3,profile=?4,notified=0,updated=unixepoch() WHERE project_id=?1 AND id=?2 AND last_turn=?5 AND status!='running'",params![start.project,start.child,start.turn,serde_json::to_string(start.profile)?,previous])?;
    if changed == 0 {
        return Ok(false);
    }
    let source: String = tx.query_row(
        "SELECT source FROM subagent_inbox WHERE child_id=?1 AND consumed=0 ORDER BY seq LIMIT 1",
        [start.child],
        |r| r.get(0),
    )?;
    let source: Value = serde_json::from_str(&source)?;
    let parent = source["senderTurnId"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Child message is missing senderTurnId"))?;
    let call = source["callId"]
        .as_str()
        .ok_or_else(|| anyhow::anyhow!("Child message is missing callId"))?;
    start.session(&tx)?;
    crate::assistant::child_activity::start(
        &tx,
        start.project,
        start.turn,
        start.child,
        parent,
        call,
        &start.profile.id,
    )?;
    tx.commit()?;
    Ok(true)
}

pub(super) fn fail(
    db: &mut Connection,
    project: &str,
    turn: &str,
    child: &str,
    error: &str,
) -> anyhow::Result<Option<Value>> {
    let tx = db.transaction()?;
    let changed = tx.execute("UPDATE subagent_runs SET status='failed',output=?4,updated=unixepoch() WHERE project_id=?1 AND id=?2 AND last_turn=?3",params![project,child,turn,error])?;
    anyhow::ensure!(changed == 1, "Child execution is no longer current");
    let activity = crate::assistant::child_activity::update(
        &tx,
        project,
        turn,
        "subagent/settled",
        &json!({"stopReason":"error"}),
    )?;
    tx.commit()?;
    Ok(activity)
}

#[cfg(test)]
#[path = "persistence_tests.rs"]
mod tests;
