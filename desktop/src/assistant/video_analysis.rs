//! A tool-free role: its editable method is supplied directly with the original video.
use super::{attachments::Reference, config::Profile, profiles::AgentProfile};
use crate::database::Store;
use anyhow::{Result, ensure};
use rusqlite::Connection;
use serde_json::Value;
use std::path::Path;

pub fn install(db: &Connection, root: &Path) -> Result<()> {
    let exists: bool = db.query_row(
        "SELECT EXISTS(SELECT 1 FROM skill_resources WHERE skill_id='video-analysis' AND path='SKILL.md')",
        [], |r| r.get(0),
    )?;
    if !exists {
        let body = std::fs::read_to_string(root.join("video-analysis/SKILL.md"))?;
        db.execute("INSERT OR IGNORE INTO skill_resources(skill_id,path,text) VALUES('video-analysis','SKILL.md',?1)", [body])?;
    }
    Ok(())
}

pub fn guidance(db: &Connection, profile: &AgentProfile) -> Result<String> {
    let page = super::skills::storage::read(
        db,
        &super::profiles::skill_setting(profile),
        "video-analysis",
        "SKILL.md",
        0,
        true,
    )?;
    Ok(format!(
        "Video analysis Skill (revision {}):\n{}",
        page["revision"],
        page["text"].as_str().unwrap_or("")
    ))
}

pub fn validate_video(store: &Store, refs: &[Reference], model: &Profile) -> Result<()> {
    if refs.is_empty() {
        return Ok(());
    }
    ensure!(
        model.inputs.video,
        "请在当前对话模型中启用视频输入，再发送原视频进行拆解"
    );
    let assets = store.assets()?;
    ensure!(
        refs.iter()
            .all(|r| r.kind == "asset" && assets.iter().any(|a| a.id == r.id && a.kind == "video")),
        "视频拆解只接收原视频，请直接引用视频文件"
    );
    Ok(())
}

pub fn isolate(snapshot: &mut Value) {
    if let Some(fields) = snapshot.as_object_mut() {
        fields.retain(|key, _| {
            ["id", "agent", "skills", "promptGuidance", "task"].contains(&key.as_str())
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            .unwrap();
        super::super::skills::storage::init(&db).unwrap();
        db
    }
    fn role() -> AgentProfile {
        super::super::profiles::builtins()
            .into_iter()
            .find(|p| p.id == "video-analyst")
            .unwrap()
    }
    #[test]
    fn install_and_reads_preserve_custom_rules_and_other_roles() {
        let db = db();
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills");
        db.execute("INSERT INTO skill_resources(skill_id,path,text) VALUES('creative-concepts','SKILL.md','CUSTOM CONCEPT')", []).unwrap();
        install(&db, &root).unwrap();
        db.execute("UPDATE skill_resources SET text='CUSTOM VIDEO METHOD', revision=7 WHERE skill_id='video-analysis'", []).unwrap();
        install(&db, &root).unwrap();
        assert!(
            guidance(&db, &role())
                .unwrap()
                .contains("CUSTOM VIDEO METHOD")
        );
        let other: String = db
            .query_row(
                "SELECT text FROM skill_resources WHERE skill_id='creative-concepts'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(other, "CUSTOM CONCEPT");
        let agents = super::super::profiles::read(&db).unwrap();
        assert!(agents.iter().any(|p| p.id == "video-analyst"));
    }
    #[test]
    fn analyst_cannot_gain_tools_or_other_skills() {
        let mut role = role();
        super::super::profiles::validate(&role).unwrap();
        for action in [
            "inspect",
            "history",
            "read_skill",
            "skills",
            "models",
            "edit",
            "delegate",
        ] {
            assert!(!super::super::profiles::allows(&role, action));
        }
        role.tool_ids.push("project-read".into());
        assert!(super::super::profiles::validate(&role).is_err());
        assert!(!super::super::profiles::allows(&role, "inspect"));
        role.tool_ids.clear();
        role.skill_ids.push("ad-script".into());
        assert!(super::super::profiles::validate(&role).is_err());
    }
    #[test]
    fn original_video_requires_configured_video_input_and_accepts_no_other_assets() {
        let (root, store, _) = super::super::attachment_tests::fixture();
        let mut model = super::super::attachment_tests::multimodal();
        model.adapter = "openai-compatible".into();
        let refs = [Reference {
            kind: "asset".into(),
            id: "video".into(),
        }];
        validate_video(&store, &refs, &model).unwrap();
        model.inputs.video = false;
        assert!(validate_video(&store, &refs, &model).is_err());
        model.inputs.video = true;
        let refs = [Reference {
            kind: "asset".into(),
            id: "image".into(),
        }];
        assert!(validate_video(&store, &refs, &model).is_err());
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn analysis_context_excludes_project_and_production_instructions() {
        let mut snapshot = json!({"id":"p","agent":super::super::model_profile::role(&role()),"skills":[],"promptGuidance":"CUSTOM VIDEO METHOD","workspace":{"script":"UNRELATED SCRIPT"},"creation":"OTHER GOAL","unverifiedResults":["OTHER RESULT"]});
        isolate(&mut snapshot);
        let context = snapshot.to_string();
        for unwanted in ["UNRELATED SCRIPT", "OTHER GOAL", "OTHER RESULT"] {
            assert!(!context.contains(unwanted));
        }
        let system = super::super::prompts::system(&snapshot);
        assert!(system.contains("CUSTOM VIDEO METHOD"));
        assert!(!system.contains("mstudio_"));
    }
}
