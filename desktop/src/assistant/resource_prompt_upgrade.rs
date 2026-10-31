//! Refresh recognized shipped resource wording without replacing authored role instructions.
use super::profiles::AgentProfile;
use anyhow::Result;
use rusqlite::{Connection, params};
use sha2::{Digest, Sha256};

const PREVIOUS: &[(&str, &str)] = &[
    (
        "coordinator",
        "eaa14343e09563b43d94fab02f9a0b015ad4b874a8c2e8a166d34e197e164515",
    ),
    (
        "concept",
        "75b5c0fd9c8ad162dc20d741110267b06517d26ea425f5d3bfc226adfd5e68de",
    ),
    (
        "writer",
        "bc3ffe0272e724740b1992dceefc4cbee1eba62f626d72aa936630677e7031a7",
    ),
    (
        "director",
        "1938a95bee29771bc13d20a2b80f0789d55c105d1be723022a397c285097f460",
    ),
    (
        "assets",
        "36bb4ec57be2fb3934f9b17b167822ea4da1249fbfd195b9ca37c9ef31f51195",
    ),
    (
        "assets",
        "1cd032dd21f6e63524124817688a13ab0b2ebf80651e3dde4a26d430aa30db52",
    ),
    (
        "image",
        "4de8297be4160a288de13b3b18b88ec6fab2562c92a43d781a9127ac4ddff0a4",
    ),
    (
        "image",
        "9a520559d985ee8254d292a99c4d8cde7f2d9bdfacd5e25d4472349e7dd03c18",
    ),
    (
        "production",
        "686a004bc905e8cbde617b061303f13a0e9e0f826d718ad999b6226e23cd7649",
    ),
    (
        "editor",
        "b89542c1bad307ed95cc2d5faaea2bf4d3563f9740d99a8abb6c48b5caffbea8",
    ),
    (
        "editor",
        "81876cd2874c12f03890e1f104350843a6cdc8cf48b4c335e5b3e3651b86b87d",
    ),
];

pub fn apply(
    db: &Connection,
    agents: &mut [AgentProfile],
    defaults: &[AgentProfile],
) -> Result<bool> {
    let mut changed = false;
    for agent in agents {
        let Some(next) = defaults.iter().find(|p| p.id == agent.id) else {
            continue;
        };
        if agent.instructions == next.instructions {
            continue;
        }
        let digest = format!("{:x}", Sha256::digest(agent.instructions.as_bytes()));
        if !PREVIOUS
            .iter()
            .any(|(id, hash)| *id == agent.id && *hash == digest)
        {
            continue;
        }
        db.execute(
            "INSERT OR IGNORE INTO settings(key,value) VALUES(?1,?2)",
            params![
                format!("agent-resource-prompt-v2:{}", agent.id),
                serde_json::to_string(agent)?
            ],
        )?;
        agent.instructions = next.instructions.clone();
        agent.revision += 1;
        changed = true;
    }
    Ok(changed)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn known_resource_prompt_updates_once_and_keeps_tools_labels_and_custom_content() {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            .unwrap();
        let old: serde_json::Value =
            serde_json::from_str(include_str!("resource_prompt_legacy.json")).unwrap();
        let defaults = super::super::profiles::builtins();
        let mut role = defaults.iter().find(|p| p.id == "assets").unwrap().clone();
        role.name = "我的资产角色".into();
        role.instructions = old["assets"].as_str().unwrap().into();
        role.tool_ids = vec!["project-read".into()];
        let before = role.clone();
        let mut agents = vec![role];
        assert!(apply(&db, &mut agents, &defaults).unwrap());
        assert_eq!(
            agents[0].instructions,
            defaults
                .iter()
                .find(|p| p.id == "assets")
                .unwrap()
                .instructions
        );
        assert_eq!(agents[0].tool_ids, before.tool_ids);
        assert_eq!(agents[0].name, before.name);
        assert_eq!(agents[0].revision, before.revision + 1);
        assert!(!apply(&db, &mut agents, &defaults).unwrap());
        let backup: String = db
            .query_row(
                "SELECT value FROM settings WHERE key='agent-resource-prompt-v2:assets'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<AgentProfile>(&backup)
                .unwrap()
                .instructions,
            before.instructions
        );
        agents[0].instructions = "My authored resource method and other professional rules".into();
        assert!(!apply(&db, &mut agents, &defaults).unwrap());
        assert_eq!(
            agents[0].instructions,
            "My authored resource method and other professional rules"
        );
    }
}
