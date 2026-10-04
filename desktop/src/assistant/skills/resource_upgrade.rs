//! Upgrade recognized defaults atomically; user-authored resources remain owned by the user.
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, path::Path};

const MARKER: &str = "concise_skill_resources_v1";
const OLD_EDITOR_CHECK: &str =
    "更新后 inspect 核实实际时间线；未播放、试听不能声称节奏或音高已验收。";
const EDITOR_CHECK: &str = "使用完整保存回执核对实际时间线；仅在字段缺失、冲突或状态不明时补读。未播放、试听不能声称节奏或音高已验收。";

#[derive(Deserialize)]
struct Manifest {
    resources: Vec<Resource>,
    agents: BTreeMap<String, String>,
}
#[derive(Deserialize)]
struct Resource {
    skill: String,
    path: String,
    previous: Option<String>,
    remove: bool,
}
fn digest(text: &str) -> String {
    format!("{:x}", Sha256::digest(text.as_bytes()))
}
pub fn migrate(db: &Connection, root: &Path) -> Result<()> {
    let manifest = serde_json::from_str(include_str!("resource_upgrade.json"))?;
    apply(db, root, &manifest)
}
fn apply(db: &Connection, root: &Path, manifest: &Manifest) -> Result<()> {
    let tx = db.unchecked_transaction()?;
    if tx.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key=?1)",
        [MARKER],
        |r| r.get::<_, bool>(0),
    )? {
        return Ok(());
    }
    for resource in &manifest.resources {
        let old: Option<String> = tx
            .query_row(
                "SELECT text FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![resource.skill, resource.path],
                |r| r.get(0),
            )
            .optional()?;
        if resource.remove {
            if old
                .as_ref()
                .is_some_and(|text| Some(digest(text)) == resource.previous)
            {
                tx.execute(
                    "DELETE FROM skill_resources WHERE skill_id=?1 AND path=?2",
                    params![resource.skill, resource.path],
                )?;
            }
            continue;
        }
        let current = std::fs::read_to_string(root.join(&resource.skill).join(&resource.path))?;
        match old {
            None if resource.previous.is_none() => {
                tx.execute(
                    "INSERT INTO skill_resources(skill_id,path,text) VALUES(?1,?2,?3)",
                    params![resource.skill, resource.path, current],
                )?;
            }
            Some(old) if old != current && Some(digest(&old)) == resource.previous => {
                tx.execute("UPDATE skill_resources SET text=?3,revision=revision+1,updated=unixepoch() WHERE skill_id=?1 AND path=?2", params![resource.skill,resource.path,current])?;
            }
            _ => {}
        }
    }
    let raw: Option<String> = tx
        .query_row("SELECT value FROM settings WHERE key='agents'", [], |r| {
            r.get(0)
        })
        .optional()?;
    if let Some(raw) = raw {
        let mut agents: Vec<super::super::profiles::AgentProfile> = serde_json::from_str(&raw)?;
        let defaults = super::super::profiles::builtins();
        let mut changed = false;
        for agent in &mut agents {
            let before = agent.instructions.clone();
            if manifest.agents.get(&agent.id) == Some(&digest(&before))
                && let Some(default) = defaults.iter().find(|p| p.id == agent.id)
            {
                agent.instructions.clone_from(&default.instructions);
            }
            if agent.id == "editor" {
                agent.instructions = agent.instructions.replace(OLD_EDITOR_CHECK, EDITOR_CHECK);
            }
            if agent.instructions != before {
                agent.revision += 1;
                changed = true;
            }
        }
        if changed {
            tx.execute(
                "UPDATE settings SET value=?1 WHERE key='agents'",
                [serde_json::to_string(&agents)?],
            )?;
        }
    }
    tx.execute(
        "INSERT INTO settings(key,value) VALUES(?1,'true')",
        [MARKER],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "resource_upgrade_tests.rs"]
mod tests;
