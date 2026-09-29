use crate::database::Store;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

pub const TOOL_IDS: [&str; 13] = [
    "agent-delegate",
    "project-read",
    "project-brief",
    "project-script",
    "project-shots",
    "project-frames",
    "project-assets",
    "project-production",
    "project-timeline",
    "project-edit",
    "media-generation",
    "memory-read",
    "memory-write",
];
pub const SKILL_IDS: [&str; 11] = [
    "image-prompt",
    "video-prompt",
    "color-grading",
    "transition-design",
    "ad-team",
    "ad-script",
    "storyboard-art",
    "asset-preparation",
    "product-storyboard",
    "creative-ad-director",
    "product-video-production",
];
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AgentProfile {
    pub id: String,
    pub name: String,
    pub description: String,
    pub instructions: String,
    pub skill_ids: Vec<String>,
    pub tool_ids: Vec<String>,
    pub enabled: bool,
    pub revision: u64,
}
pub fn builtins() -> Vec<AgentProfile> {
    serde_json::from_str(include_str!("../../../frontend/src/agents/defaults.json"))
        .expect("valid built-in agents")
}
#[cfg(test)]
pub fn defaults(_: &str) -> AgentProfile {
    builtins().remove(0)
}
pub fn read(db: &rusqlite::Connection) -> Result<Vec<AgentProfile>> {
    match crate::models::setting(db, "agents")? {
        Some(raw) => {
            let mut saved: Vec<AgentProfile> = serde_json::from_str(&raw)?;
            let original_len = saved.len();
            for builtin in builtins() {
                if !saved.iter().any(|p| p.id == builtin.id)
                    && [
                        "storyboard-artist",
                        "colorist",
                        "transition-designer",
                        "asset-designer",
                    ]
                    .contains(&builtin.id.as_str())
                {
                    saved.push(builtin);
                }
            }
            if saved.len() != original_len {
                db.execute(
                    "UPDATE settings SET value=?1 WHERE key='agents'",
                    [serde_json::to_string(&saved)?],
                )?;
            }
            Ok(saved)
        }
        None => {
            let defaults = builtins();
            db.execute(
                "INSERT INTO settings(key,value) VALUES('agents',?1)",
                [serde_json::to_string(&defaults)?],
            )?;
            Ok(defaults)
        }
    }
}
pub fn resolve(db: &rusqlite::Connection, id: Option<&str>) -> Result<AgentProfile> {
    let profile = read(db)?
        .into_iter()
        .find(|p| p.id == id.unwrap_or("coordinator"))
        .ok_or_else(|| anyhow::anyhow!("Agent 不存在，请重新选择"))?;
    ensure!(profile.enabled, "Agent 已停用");
    Ok(profile)
}
pub fn validate(profile: &AgentProfile) -> Result<()> {
    ensure!(
        !profile.id.is_empty() && profile.id.len() <= 80,
        "Agent ID 无效"
    );
    ensure!(
        !profile.name.trim().is_empty() && profile.name.chars().count() <= 80,
        "Agent 名称须为 1–80 字"
    );
    ensure!(
        profile.description.chars().count() <= 300 && profile.instructions.chars().count() <= 6000,
        "Agent 描述或指令过长"
    );
    ensure!(
        profile.skill_ids.len() <= SKILL_IDS.len()
            && profile
                .skill_ids
                .iter()
                .all(|id| SKILL_IDS.contains(&id.as_str())),
        "Agent 包含未安装的 Skill"
    );
    ensure!(
        profile.tool_ids.len() <= TOOL_IDS.len()
            && profile
                .tool_ids
                .iter()
                .all(|id| TOOL_IDS.contains(&id.as_str())),
        "Agent 包含未安装的工具"
    );
    ensure!(
        !profile
            .tool_ids
            .iter()
            .any(|id| id.starts_with("project-") && id != "project-read")
            || profile.tool_ids.contains(&"project-read".into()),
        "编辑工具依赖读取项目工具"
    );
    ensure!(
        !profile.tool_ids.contains(&"memory-write".into())
            || profile.tool_ids.contains(&"memory-read".into()),
        "整理记忆依赖读取记忆工具"
    );
    Ok(())
}
pub fn save(db: &rusqlite::Connection, mut profile: AgentProfile) -> Result<()> {
    validate(&profile)?;
    let mut agents = read(db)?;
    profile.revision = agents
        .iter()
        .find(|a| a.id == profile.id)
        .map_or(1, |a| a.revision + 1);
    if let Some(old) = agents.iter_mut().find(|a| a.id == profile.id) {
        *old = profile;
    } else {
        ensure!(agents.len() < 50, "最多保存 50 个 Agent");
        agents.push(profile);
    }
    db.execute("INSERT INTO settings VALUES('agents',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_string(&agents)?])?;
    Ok(())
}
pub fn skill_setting(profile: &AgentProfile) -> String {
    serde_json::to_string(
        &profile
            .skill_ids
            .iter()
            .map(|id| format!("skill-{id}"))
            .collect::<Vec<_>>(),
    )
    .unwrap()
}
pub fn allows(profile: &AgentProfile, action: &str) -> bool {
    let has = |id: &str| profile.tool_ids.iter().any(|v| v == id);
    match action {
        "delegate" => has("agent-delegate"),
        "skills" | "read_skill" => !profile.skill_ids.is_empty(),
        "inspect" | "history" => has("project-read"),
        "models" => has("media-generation"),
        "memory-read" => has("memory-read"),
        "memory-write" => has("memory-read") && has("memory-write"),
        "edit" => {
            has("project-read")
                && profile
                    .tool_ids
                    .iter()
                    .any(|id| id.starts_with("project-") && id != "project-read")
        }
        _ => false,
    }
}
#[tauri::command]
pub fn agent_catalog(store: State<Store>) -> Result<Vec<AgentProfile>, String> {
    read(&store.db.lock().unwrap()).map_err(|e| e.to_string())
}
#[tauri::command]
pub fn save_agent(
    app: tauri::AppHandle,
    store: State<Store>,
    profile: AgentProfile,
) -> Result<(), String> {
    save(&store.db.lock().unwrap(), profile).map_err(|e| e.to_string())?;
    // Notify every open window after the durable update; notification failure does not undo it.
    let _ = app.emit("mstudio-agents-changed", ());
    Ok(())
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn agent_skills_are_scoped_and_edit_requires_read() {
        let mut p = defaults("");
        p.skill_ids = vec!["product-storyboard".into()];
        p.tool_ids.clear();
        assert!(!allows(&p, "edit"));
        assert!(!allows(&p, "inspect"));
        assert_eq!(skill_setting(&p), "[\"skill-product-storyboard\"]");
        p.tool_ids.push("project-edit".into());
        assert!(validate(&p).is_err());
        p.tool_ids.push("project-read".into());
        assert!(validate(&p).is_ok());
        assert!(allows(&p, "edit"));
    }
    #[test]
    fn tools_and_skills_are_independent_and_revocation_changes_schema() {
        let mut p = builtins()
            .into_iter()
            .find(|p| p.id == "production")
            .unwrap();
        p.tool_ids.clear();
        assert!(allows(&p, "read_skill"));
        assert!(!allows(&p, "inspect"));
        assert!(!allows(&p, "edit"));
        assert!(!super::super::permissions::allows_operation(
            &p,
            "request_generation"
        ));
        let schema = super::super::tool_schema::for_profile(&p);
        assert_eq!(
            schema["properties"]["action"]["enum"],
            serde_json::json!(["skills", "read_skill"])
        );
        p.skill_ids.clear();
        p.tool_ids = vec!["project-read".into(), "project-script".into()];
        assert!(allows(&p, "edit"));
        assert!(!allows(&p, "read_skill"));
        assert!(!allows(&p, "memory-write"));
        p.tool_ids.push("memory-write".into());
        assert!(validate(&p).is_err());
        p.tool_ids.push("memory-read".into());
        assert!(validate(&p).is_ok());
        assert!(allows(&p, "memory-write"));
        p.skill_ids.push("project-read".into());
        assert!(validate(&p).is_err());
    }
    #[test]
    fn agent_profiles_persist_independently_and_revision_changes() {
        let root = std::env::temp_dir().join(format!("mstudio-agent-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        let db = store.db.lock().unwrap();
        let mut first = defaults("");
        first.instructions = "Creative direction".into();
        save(&db, first.clone()).unwrap();
        let mut other = first.clone();
        other.id = "other".into();
        other.skill_ids.clear();
        save(&db, other).unwrap();
        assert_eq!(read(&db).unwrap().len(), builtins().len() + 1);
        assert_eq!(resolve(&db, None).unwrap().revision, first.revision + 1);
        assert!(resolve(&db, Some("other")).is_ok());
        first.enabled = false;
        save(&db, first).unwrap();
        assert!(resolve(&db, None).is_err());
        drop(db);
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
