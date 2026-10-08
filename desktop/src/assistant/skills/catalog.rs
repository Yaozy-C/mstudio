//! Install the consolidated Skill catalog once, with a recoverable backup.
//!
//! The shipped catalog is the single source of truth for a fresh install; this
//! module brings an already-seeded database to the same state without losing the
//! user's own Agent text. Rule bodies of shipped Skills are replaced by the
//! bundled documents, retired Skills are removed, and saved Agents are remapped.
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, params};
use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

const MARKER: &str = "skill_catalog_v5";

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Manifest {
    /// Skill ids replaced by a merged capability; their rows are removed.
    removed: Vec<String>,
    /// Retired Agent ids whose duties now live in a shipped profile.
    retired_agents: Vec<String>,
    /// Old skill id -> shipped skill id, or "" when the capability was dropped.
    skill_map: BTreeMap<String, String>,
    /// Targeted corrections to stored model rules, e.g. an unavailable action.
    model_rule_edits: Vec<ModelRuleEdit>,
}

#[derive(Deserialize)]
struct ModelRuleEdit {
    id: String,
    before: String,
    after: String,
}

pub fn migrate(db: &Connection, root: &Path) -> Result<()> {
    if db.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key=?1)",
        [MARKER],
        |r| r.get::<_, bool>(0),
    )? {
        return Ok(());
    }
    let manifest: Manifest = serde_json::from_str(include_str!("catalog_migration.json"))?;
    // Only a database that still holds superseded packages has anything to lose.
    let mut legacy = false;
    for id in &manifest.removed {
        let present: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM skill_resources WHERE skill_id=?1)",
            [id],
            |r| r.get(0),
        )?;
        if present {
            legacy = true;
            break;
        }
    }
    let backup = if legacy { backup(db)? } else { String::new() };
    let tx = db.unchecked_transaction()?;
    install_shipped(&tx, root)?;
    for id in &manifest.removed {
        tx.execute("DELETE FROM skill_resources WHERE skill_id=?1", [id])?;
    }
    correct_model_rules(&tx, &manifest.model_rule_edits)?;
    rewrite_agents(&tx, &manifest)?;
    tx.execute(
        "INSERT INTO settings(key,value) VALUES(?1,?2)",
        params![MARKER, backup],
    )?;
    tx.commit()?;
    Ok(())
}

fn backup(db: &Connection) -> Result<String> {
    let Some(path) = db.path().filter(|p| !p.is_empty()) else {
        return Ok(String::new());
    };
    let dest = format!(
        "{path}.before-skill-catalog-v5-{}.sqlite3",
        mstudio::media::id()
    );
    db.execute("VACUUM INTO ?1", [&dest])?;
    Ok(dest)
}

/// Copy every bundled document into the database and drop stale rows so the
/// stored catalog matches the shipped one exactly.
fn install_shipped(db: &Connection, root: &Path) -> Result<()> {
    for (id, _, _) in super::SKILLS {
        let base = root.join(id);
        ensure!(
            base.join("SKILL.md").is_file(),
            "Missing default rules: {id}"
        );
        let mut paths = BTreeSet::new();
        collect(&base, &base, &mut paths)?;
        ensure!(!paths.is_empty(), "No rule documents for {id}");
        for path in &paths {
            let text = std::fs::read_to_string(base.join(path))?;
            ensure!(text.len() <= 200_000, "Rule file too large: {id}/{path}");
            db.execute(
                "INSERT INTO skill_resources(skill_id,path,text,revision) VALUES(?1,?2,?3,1)
                 ON CONFLICT(skill_id,path) DO UPDATE SET text=excluded.text,
                 revision=revision+1,updated=unixepoch()
                 WHERE skill_resources.text!=excluded.text",
                params![id, path, text],
            )?;
        }
        let stored: Vec<String> = db
            .prepare("SELECT path FROM skill_resources WHERE skill_id=?1")?
            .query_map([id], |r| r.get(0))?
            .collect::<rusqlite::Result<_>>()?;
        for path in stored.iter().filter(|p| !paths.contains(*p)) {
            db.execute(
                "DELETE FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![id, path],
            )?;
        }
    }
    Ok(())
}

/// Apply targeted corrections to stored model rules without replacing user edits.
fn correct_model_rules(db: &Connection, edits: &[ModelRuleEdit]) -> Result<()> {
    for edit in edits {
        let stored: Option<String> = db
            .query_row(
                "SELECT text FROM model_prompt_rules WHERE id=?1",
                [&edit.id],
                |r| r.get(0),
            )
            .ok();
        if let Some(stored) = stored {
            let corrected = stored.replace(&edit.before, &edit.after);
            if corrected != stored {
                db.execute(
                    "UPDATE model_prompt_rules SET text=?2,revision=revision+1 WHERE id=?1",
                    params![edit.id, corrected],
                )?;
            }
        }
    }
    Ok(())
}

fn collect(base: &Path, dir: &Path, paths: &mut BTreeSet<String>) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        ensure!(
            !entry.file_type()?.is_symlink(),
            "Rule resources cannot be symlinks"
        );
        let path = entry.path();
        if path.is_dir() {
            collect(base, &path, paths)?;
        } else if path.extension().and_then(|v| v.to_str()) == Some("md") {
            paths.insert(
                path.strip_prefix(base)?
                    .components()
                    .map(|p| p.as_os_str().to_string_lossy())
                    .collect::<Vec<_>>()
                    .join("/"),
            );
        }
    }
    Ok(())
}

/// Keep the shipped roster. A profile that still exists keeps its identity,
/// enablement and authored instructions, but adopts the shipped capabilities
/// because retired Skill ids can no longer be read. Retired profiles are
/// removed; unknown profiles survive with their capability list remapped.
fn rewrite_agents(db: &Connection, manifest: &Manifest) -> Result<()> {
    let Some(raw) = crate::models::setting(db, "agents")? else {
        return Ok(());
    };
    let saved: Vec<super::super::profiles::AgentProfile> = serde_json::from_str(&raw)?;
    let defaults = super::super::profiles::builtins();
    let mut next = Vec::new();
    for default in &defaults {
        match saved.iter().find(|p| p.id == default.id) {
            Some(existing) => {
                let mut kept = existing.clone();
                kept.name.clone_from(&existing.name);
                kept.enabled = existing.enabled;
                if kept.skill_ids != default.skill_ids || kept.tool_ids != default.tool_ids {
                    kept.skill_ids.clone_from(&default.skill_ids);
                    kept.tool_ids.clone_from(&default.tool_ids);
                    kept.revision += 1;
                }
                next.push(kept);
            }
            None => next.push(default.clone()),
        }
    }
    let custom: Vec<_> = saved
        .iter()
        .filter(|p| {
            !defaults.iter().any(|d| d.id == p.id)
                && !manifest.retired_agents.contains(&p.id)
                && !next.iter().any(|n| n.id == p.id)
        })
        .cloned()
        .collect();
    for profile in custom {
        let mut kept = profile;
        let mapped: Vec<String> = kept
            .skill_ids
            .iter()
            .filter_map(|id| manifest.skill_map.get(id))
            .filter(|id| !id.is_empty())
            .cloned()
            .collect();
        if mapped != kept.skill_ids {
            kept.skill_ids = mapped;
            kept.revision += 1;
        }
        next.push(kept);
    }
    if next.len() > 50 {
        next.truncate(50);
    }
    db.execute(
        "UPDATE settings SET value=?1 WHERE key='agents'",
        [serde_json::to_string(&next).context("serialize agents")?],
    )?;
    Ok(())
}

#[cfg(test)]
#[path = "catalog_tests.rs"]
mod tests;
