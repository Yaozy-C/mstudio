//! Add the board workflow without replacing or deleting legacy rule bodies.
use anyhow::Result;
use rusqlite::{Connection, params};
use std::path::Path;

const MARKER: &str = "temporal_storyboard_skills_v1";
const PACKAGES: [&str; 2] = ["storyboard-image-production", "storyboard-video-production"];

pub(super) fn migrate(db: &Connection, root: &Path) -> Result<()> {
    if db.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key=?1)",
        [MARKER],
        |r| r.get::<_, bool>(0),
    )? {
        return Ok(());
    }
    let tx = db.unchecked_transaction()?;
    for id in PACKAGES {
        let base = root.join(id);
        super::storage::seed_directory(&tx, id, &base, &base)?;
    }
    if let Some(raw) = crate::models::setting(&tx, "agents")? {
        let mut agents: Vec<crate::assistant::profiles::AgentProfile> = serde_json::from_str(&raw)?;
        tx.execute(
            "INSERT INTO settings(key,value) VALUES(?1,?2)",
            params![format!("{MARKER}_agents_backup"), raw],
        )?;
        crate::assistant::profile_instructions::upgrade(
            &mut agents,
            &crate::assistant::profiles::builtins(),
        );
        for agent in &mut agents {
            let mut changed = false;
            for skill in &mut agent.skill_ids {
                let next = match skill.as_str() {
                    "image-production" => PACKAGES[0],
                    "product-video-production" => PACKAGES[1],
                    _ => continue,
                };
                *skill = next.into();
                changed = true;
            }
            if changed {
                let mut seen = std::collections::BTreeSet::new();
                agent.skill_ids.retain(|id| seen.insert(id.clone()));
                agent.instructions = agent
                    .instructions
                    .replace("Read image-production", "Read storyboard-image-production")
                    .replace(
                        "Read product-video-production",
                        "Read storyboard-video-production",
                    );
                agent.revision += 1;
            }
        }
        tx.execute(
            "UPDATE settings SET value=?1 WHERE key='agents'",
            [serde_json::to_string(&agents)?],
        )?;
    }
    tx.execute(
        "INSERT INTO settings(key,value) VALUES(?1,'true')",
        [MARKER],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "storyboard_tests.rs"]
mod tests;
