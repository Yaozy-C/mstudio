use super::{SKILLS, dependency, enabled};
use anyhow::{Context, Result, ensure};
use rusqlite::{Connection, OptionalExtension, params};
use serde_json::{Value, json};
use std::path::{Component, Path};

pub fn init(db: &Connection) -> Result<()> {
    db.execute_batch(
        "CREATE TABLE IF NOT EXISTS skill_resources(
        skill_id TEXT NOT NULL, path TEXT NOT NULL, text TEXT NOT NULL,
        revision INTEGER NOT NULL DEFAULT 1, updated INTEGER NOT NULL DEFAULT(unixepoch()),
        PRIMARY KEY(skill_id,path));",
    )?;
    Ok(())
}
pub fn initialized(db: &Connection) -> Result<bool> {
    Ok(db.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key='skills_initialized')",
        [],
        |r| r.get(0),
    )?)
}
pub fn seed(db: &Connection, root: &Path) -> Result<()> {
    init(db)?;
    if initialized(db)? {
        return Ok(());
    }
    let tx = db.unchecked_transaction()?;
    for (id, _, _) in SKILLS {
        let base = root.join(id);
        ensure!(
            base.join("SKILL.md").is_file(),
            "Missing default rules: {id}"
        );
        seed_directory(&tx, id, &base, &base)?;
    }
    tx.execute(
        "INSERT INTO settings(key,value) VALUES('skills_initialized','true')",
        [],
    )?;
    tx.commit()?;
    Ok(())
}
fn seed_directory(db: &Connection, id: &str, base: &Path, dir: &Path) -> Result<()> {
    for entry in std::fs::read_dir(dir)? {
        let entry = entry?;
        ensure!(
            !entry.file_type()?.is_symlink(),
            "Rule resources cannot be symlinks"
        );
        let path = entry.path();
        if path.is_dir() {
            seed_directory(db, id, base, &path)?;
        } else if path.extension().and_then(|v| v.to_str()) == Some("md") {
            let text = std::fs::read_to_string(&path)?;
            ensure!(text.len() <= 200_000, "Rule file too large");
            db.execute(
                "INSERT OR IGNORE INTO skill_resources(skill_id,path,text) VALUES(?1,?2,?3)",
                params![
                    id,
                    path.strip_prefix(base)?
                        .components()
                        .map(|p| p.as_os_str().to_string_lossy())
                        .collect::<Vec<_>>()
                        .join("/"),
                    text
                ],
            )?;
        }
    }
    Ok(())
}
fn resolve(id: &str, resource: &str) -> Result<(String, String)> {
    ensure!(
        SKILLS.iter().any(|s| s.0 == id),
        "Creative Skill not installed"
    );
    let resource = resource.split('#').next().unwrap_or("SKILL.md");
    let mut parts = vec![id.to_owned()];
    for part in Path::new(resource).components() {
        match part {
            Component::Normal(v) => parts.push(v.to_string_lossy().into_owned()),
            Component::CurDir => {}
            Component::ParentDir => {
                ensure!(!parts.is_empty(), "Rule path escapes its root");
                parts.pop();
            }
            _ => anyhow::bail!("Use a relative path within the Skill"),
        }
    }
    ensure!(
        parts.len() >= 2 && SKILLS.iter().any(|s| s.0 == parts[0]),
        "Rule path outside installed Skills"
    );
    let owner = parts.remove(0);
    let path = parts.join("/");
    ensure!(path.ends_with(".md"), "Only Markdown rules supported");
    Ok((owner, path))
}
pub fn catalog(db: &Connection, setting: &str) -> Result<Value> {
    let mut items = vec![];
    for (id, name, description) in SKILLS {
        let available: bool = db.query_row(
            "SELECT EXISTS(SELECT 1 FROM skill_resources WHERE skill_id=?1 AND path='SKILL.md')",
            [id],
            |r| r.get(0),
        )?;
        let revision: i64 = db.query_row(
            "SELECT coalesce(sum(revision),0) FROM skill_resources WHERE skill_id=?1",
            [id],
            |r| r.get(0),
        )?;
        items.push(json!({"id":id,"name":name,"description":description,
            "available":available,"enabled":enabled(setting,id)?,"path":"SKILL.md","storage":"database","revision":revision}));
    }
    Ok(json!(items))
}
pub fn read(
    db: &Connection,
    setting: &str,
    id: &str,
    resource: &str,
    offset: usize,
    active_only: bool,
) -> Result<Value> {
    let (owner, path) = resolve(id, resource)?;
    ensure!(!active_only || enabled(setting, id)?, "Skill disabled");
    ensure!(
        !active_only || enabled(setting, &owner)? || dependency(id, &owner),
        "Referenced Skill disabled"
    );
    let (text, revision): (String, i64) = db
        .query_row(
            "SELECT text,revision FROM skill_resources WHERE skill_id=?1 AND path=?2",
            params![owner, path],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?
        .context("Rule document not found")?;
    let mut stmt = db.prepare(
        "SELECT path FROM skill_resources WHERE skill_id=?1 AND path!='SKILL.md' ORDER BY path",
    )?;
    let resources = stmt
        .query_map([&owner], |r| r.get::<_, String>(0))?
        .collect::<rusqlite::Result<Vec<_>>>()?;
    let chars: Vec<_> = text.chars().collect();
    let start = offset.min(chars.len());
    let end = chars.len();
    Ok(
        json!({"skill":owner,"path":path,"text":chars[start..end].iter().collect::<String>(),
        "revision":revision,"offset":start,"nextOffset":if end<chars.len(){Some(end)}else{None},
        "totalCharacters":chars.len(),"resources":resources}),
    )
}
pub fn save(db: &Connection, id: &str, path: &str, text: &str, revision: i64) -> Result<Value> {
    let (owner, path) = resolve(id, path)?;
    ensure!(owner == id, "请在所属 skill 内编辑文档");
    ensure!(
        !text.trim().is_empty() && text.len() <= 200_000,
        "规则正文为空或超过 200 KB"
    );
    let count = db.execute(
        "UPDATE skill_resources SET text=?3,revision=revision+1,updated=unixepoch() WHERE skill_id=?1 AND path=?2 AND revision=?4",
        params![owner,path,text,revision])?;
    ensure!(count == 1, "规则已被修改或不存在，请重新加载后再保存");
    Ok(json!({"skill":owner,"path":path,"revision":revision+1}))
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;
