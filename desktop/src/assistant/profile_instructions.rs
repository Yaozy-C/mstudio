//! Upgrade only recognized shipped instruction bodies; preserve user customizations.
use super::profiles::AgentProfile;
use std::collections::BTreeMap;

pub(super) fn upgrade(saved: &mut [AgentProfile], builtins: &[AgentProfile]) -> bool {
    let baseline: BTreeMap<String, String> =
        serde_json::from_str(include_str!("profile_instruction_baseline.json"))
            .expect("valid shipped instruction baseline");
    let mut changed = false;
    for profile in saved {
        let Some(old) = baseline.get(&profile.id) else {
            continue;
        };
        let Some(current) = builtins.iter().find(|p| p.id == profile.id) else {
            continue;
        };
        if profile.instructions == *old && profile.instructions != current.instructions {
            profile.instructions.clone_from(&current.instructions);
            profile.revision += 1;
            changed = true;
        }
    }
    changed
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{assistant::profiles, database::Store};
    #[test]
    fn upgrades_recognized_instructions_once_without_replacing_user_configuration() {
        let root =
            std::env::temp_dir().join(format!("mstudio-prompt-upgrade-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        let db = store.db.lock().unwrap();
        let baseline: BTreeMap<String, String> =
            serde_json::from_str(include_str!("profile_instruction_baseline.json")).unwrap();
        let mut saved = profiles::builtins();
        let editor = saved.iter_mut().find(|p| p.id == "editor").unwrap();
        editor.instructions = baseline["editor"].clone();
        editor.name = "My editor".into();
        editor.enabled = false;
        editor.tool_ids = vec!["project-read".into()];
        editor.revision = 100;
        let custom = saved.iter_mut().find(|p| p.id == "concept").unwrap();
        custom.instructions = "我的自定义要求：不要覆盖".into();
        db.execute("INSERT INTO settings(key,value) VALUES('agents',?1) ON CONFLICT(key) DO UPDATE SET value=excluded.value", [serde_json::to_string(&saved).unwrap()]).unwrap();
        let first = profiles::read(&db).unwrap();
        let editor = first.iter().find(|p| p.id == "editor").unwrap();
        assert_eq!(editor.name, "My editor");
        assert!(!editor.enabled);
        assert_eq!(editor.tool_ids, ["project-read"]);
        assert_eq!(editor.revision, 101);
        assert!(
            editor
                .instructions
                .contains("complete authoritative receipts")
        );
        assert_eq!(
            first
                .iter()
                .find(|p| p.id == "concept")
                .unwrap()
                .instructions,
            "我的自定义要求：不要覆盖"
        );
        assert_eq!(
            serde_json::to_value(&first).unwrap(),
            serde_json::to_value(profiles::read(&db).unwrap()).unwrap()
        );
        drop(db);
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
