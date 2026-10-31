use crate::database::Store;
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
use tauri::{Emitter, State};

pub const TOOL_IDS: [&str; 14] = [
    "agent-delegate",
    "project-read",
    "media-read",
    "material-read",
    "project-brief",
    "project-script",
    "project-concepts",
    "project-shots",
    "project-frames",
    "project-assets",
    "project-production",
    "project-timeline",
    "project-edit",
    "media-generation",
];
pub const SKILL_IDS: [&str; 9] = [
    "video-analysis",
    "creative-concepts",
    "ad-script",
    "creative-ad-director",
    "image-production",
    "product-video-production",
    "video-editing",
    "storyboard-image-production",
    "storyboard-video-production",
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
    let raw = crate::models::setting(db, "agents")?;
    let mut agents: Vec<AgentProfile> = match &raw {
        Some(raw) => serde_json::from_str(raw)?,
        None => builtins(),
    };
    // Upgrade the tool assignment once; later user edits remain authoritative.
    let migrate = crate::models::setting(db, "material-reader-tools-v1")?.is_none();
    if migrate {
        for agent in &mut agents {
            if matches!(agent.id.as_str(), "concept" | "assets" | "writer")
                && agent.tool_ids.iter().any(|id| id == "project-read")
                && !agent.tool_ids.iter().any(|id| id == "material-read")
            {
                agent.tool_ids.push("material-read".into());
                agent.revision += 1;
            }
        }
    }
    let prompt_changed = super::resource_prompt_upgrade::apply(db, &mut agents, &builtins())?;
    if raw.is_none() || migrate || prompt_changed {
        db.execute("INSERT INTO settings(key,value) VALUES('agents',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value",
            [serde_json::to_string(&agents)?])?;
    }
    if migrate {
        db.execute(
            "INSERT INTO settings(key,value) VALUES('material-reader-tools-v1','1')",
            [],
        )?;
    }
    Ok(agents)
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
    if profile.id == "video-analyst" {
        ensure!(profile.tool_ids.is_empty(), "视频拆解角色不使用工具");
        ensure!(
            profile.skill_ids == ["video-analysis"],
            "视频拆解角色只使用视频拆解 Skill"
        );
    }
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
            .any(|id| (id.starts_with("project-") && id != "project-read")
                || id == "media-read"
                || id == "material-read")
            || profile.tool_ids.contains(&"project-read".into()),
        "项目编辑及媒体读取工具依赖读取项目工具"
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
    if profile.id == "video-analyst" {
        return false;
    }
    let has = |id: &str| profile.tool_ids.iter().any(|v| v == id);
    match action {
        "delegate" => has("agent-delegate"),
        "skills" | "read_skill" => !profile.skill_ids.is_empty(),
        "inspect" | "history" => has("project-read"),
        "read_media" => has("project-read") && has("media-read"),
        "models" => has("media-generation"),
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
#[path = "profiles_tests.rs"]
mod tests;
