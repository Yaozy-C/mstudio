//! Replace shipped research documents with operational rules; archive retired DB text.
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension, params};
use serde::Deserialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::path::Path;

const MARKER: &str = "creative_rule_summary_v1";
#[derive(Deserialize)]
struct Resource {
    skill: String,
    path: String,
    #[serde(default)]
    before: String,
}
#[derive(Deserialize)]
struct Manifest {
    updates: Vec<Resource>,
    retired: Vec<Resource>,
}
pub(super) fn migrate(db: &Connection, root: &Path) -> Result<()> {
    let manifest = serde_json::from_str(include_str!("rule_summary_migration.json"))?;
    apply(db, root, manifest, MARKER)
}
pub(super) fn migrate_methods(db: &Connection, root: &Path) -> Result<()> {
    let manifest = serde_json::from_str(include_str!("method_rules_migration.json"))?;
    apply(db, root, manifest, "creative_method_rules_v1")
}
pub(super) fn migrate_video_motion(db: &Connection, root: &Path) -> Result<()> {
    let manifest = serde_json::from_str(include_str!("video_motion_rules_migration.json"))?;
    apply(db, root, manifest, "video_motion_rules_v1")
}
pub(super) fn migrate_image_storyboard(db: &Connection, root: &Path) -> Result<()> {
    let manifest = serde_json::from_str(include_str!("image_storyboard_rules_migration.json"))?;
    apply(db, root, manifest, "image_storyboard_rules_v1")
}
pub(super) fn migrate_image_cases(db: &Connection, root: &Path) -> Result<()> {
    let manifest = serde_json::from_str(include_str!("image_case_rules_migration.json"))?;
    apply(db, root, manifest, "image_case_rules_v1")
}
pub(super) fn migrate_board_regeneration(db: &Connection, root: &Path) -> Result<()> {
    let manifest = serde_json::from_str(include_str!("board_regeneration_migration.json"))?;
    apply(db, root, manifest, "board_regeneration_rules_v1")
}
pub(super) fn migrate_pure_rules(db: &Connection, root: &Path) -> Result<()> {
    let manifest = serde_json::from_str(include_str!("pure_rules_migration.json"))?;
    apply(db, root, manifest, "creative_pure_rules_v1")
}
pub(super) fn migrate_object_tools(db: &Connection, root: &Path) -> Result<()> {
    let manifest = serde_json::from_str(include_str!("object_tools_migration.json"))?;
    apply(db, root, manifest, "creative_object_tools_v1")
}
fn apply(db: &Connection, root: &Path, manifest: Manifest, marker: &str) -> Result<()> {
    if db.query_row(
        "SELECT EXISTS(SELECT 1 FROM settings WHERE key=?1)",
        [marker],
        |r| r.get::<_, bool>(0),
    )? {
        return Ok(());
    }
    let tx = db.unchecked_transaction()?;
    let mut backup: Vec<Value> = vec![];
    for resource in manifest.updates.iter().chain(&manifest.retired) {
        if let Some((text, revision)) = tx
            .query_row(
                "SELECT text,revision FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![resource.skill, resource.path],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
            )
            .optional()?
        {
            backup.push(json!({"skill":resource.skill,"path":resource.path,"text":text,"revision":revision}));
        }
    }
    tx.execute(
        "INSERT INTO settings(key,value) VALUES(?1,?2)",
        params![format!("{marker}_backup"), serde_json::to_string(&backup)?],
    )?;
    for resource in &manifest.updates {
        let old: Option<String> = tx
            .query_row(
                "SELECT text FROM skill_resources WHERE skill_id=?1 AND path=?2",
                params![resource.skill, resource.path],
                |r| r.get(0),
            )
            .optional()?;
        if let Some(old) = old {
            let replacement =
                std::fs::read_to_string(root.join(&resource.skill).join(&resource.path))?;
            let next = if format!("{:x}", Sha256::digest(old.as_bytes())) == resource.before {
                replacement
            } else {
                // Keep authored entries, but remove links to the explicitly retired documents.
                let cleaned = old
                    .replace(
                        "[research](references/research.md)",
                        "the evidence rules below",
                    )
                    .replace(
                        "[research basis](references/sources.md)",
                        "authoring provenance outside the Skill",
                    );
                if cleaned != old {
                    let heading = if resource.skill == "creative-concepts" {
                        "## Use evidence to change a creative decision"
                    } else {
                        "## Keep evidence and realization separate"
                    };
                    let section = replacement
                        .split_once(heading)
                        .map(|(_, tail)| tail.split("\n## ").next().unwrap_or(tail))
                        .unwrap_or_default();
                    format!("{cleaned}\n\n{heading}{section}\n")
                } else {
                    old.clone()
                }
            };
            if next != old {
                tx.execute("UPDATE skill_resources SET text=?3,revision=revision+1,updated=unixepoch() WHERE skill_id=?1 AND path=?2", params![resource.skill, resource.path, next])?;
            }
        }
    }
    for resource in &manifest.retired {
        tx.execute(
            "DELETE FROM skill_resources WHERE skill_id=?1 AND path=?2",
            params![resource.skill, resource.path],
        )?;
    }
    tx.execute(
        "INSERT INTO settings(key,value) VALUES(?1,'true')",
        [marker],
    )?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn db() -> Connection {
        let db = Connection::open_in_memory().unwrap();
        db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT NOT NULL)")
            .unwrap();
        super::super::storage::init(&db).unwrap();
        db
    }
    fn root() -> std::path::PathBuf {
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../skills")
    }
    #[test]
    fn pure_rules_upgrade_replaces_shipped_audits_preserves_custom_text_and_runs_once() {
        let db = db();
        let previous = r#"# Inspection evidence

For each conclusion record shot ID, assetId, inspected object, time range/image region, actual observation and pass/fail/unchecked in the existing handoff or reply. Prompts, metadata and completion status are not visual evidence.

- Static: original product parts, frame state, camera and agreement with the plan.
- Motion: source, initial support, path, decisive contact, destination, counts and positions.
- Sequence: viewing changes, repetition, recognition time, speed and actual cuts.
- Sound: actually listened range, sync, material and level; no audio input means unchecked.
- Technical: only specifications returned by tools or actually inspected; technical validity is not content validity.

Reinspect affected areas and joins after repairs, retiming or new versions. Do not relabel a known essential failure as unchecked to pass it. Report capability gaps precisely. In Mstudio, project fields and messages carry these records; do not require review.json or unavailable external scripts.
"#;
        db.execute("INSERT INTO skill_resources(skill_id,path,text,revision) VALUES('product-video-production','references/evidence.md',?1,3),('image-production','references/frame-checks.md','CUSTOM USER RULES',7)", [previous]).unwrap();
        migrate_pure_rules(&db, &root()).unwrap();
        let read = |skill, path| {
            db.query_row(
                "SELECT text,revision FROM skill_resources WHERE skill_id=?1 AND path=?2",
                [skill, path],
                |r| Ok((r.get::<_, String>(0)?, r.get::<_, i64>(1)?)),
            )
            .unwrap()
        };
        let upgraded = read("product-video-production", "references/evidence.md");
        assert_eq!(
            upgraded.0,
            std::fs::read_to_string(root().join("product-video-production/references/evidence.md"))
                .unwrap()
        );
        assert_eq!(upgraded.1, 4);
        assert_eq!(
            read("image-production", "references/frame-checks.md"),
            ("CUSTOM USER RULES".into(), 7)
        );
        migrate_pure_rules(&db, &root()).unwrap();
        assert_eq!(
            read("product-video-production", "references/evidence.md"),
            upgraded
        );
    }
    #[test]
    fn replaces_recognized_entries_archives_retired_custom_text_and_is_idempotent() {
        let db = db();
        db.execute("INSERT INTO skill_resources(skill_id,path,text) VALUES('creative-concepts','SKILL.md','OLD ENTRY'),('creative-concepts','references/research.md','CUSTOM RESEARCH')", []).unwrap();
        let manifest = Manifest {
            updates: vec![Resource {
                skill: "creative-concepts".into(),
                path: "SKILL.md".into(),
                before: format!("{:x}", Sha256::digest(b"OLD ENTRY")),
            }],
            retired: vec![Resource {
                skill: "creative-concepts".into(),
                path: "references/research.md".into(),
                before: String::new(),
            }],
        };
        apply(&db, &root(), manifest, MARKER).unwrap();
        let current =
            super::super::storage::read(&db, "", "creative-concepts", "SKILL.md", 0, true).unwrap();
        assert_eq!(
            current["text"],
            std::fs::read_to_string(root().join("creative-concepts/SKILL.md")).unwrap()
        );
        assert!(
            super::super::storage::read(
                &db,
                "",
                "creative-concepts",
                "references/research.md",
                0,
                true
            )
            .is_err()
        );
        let backup: String = db
            .query_row(
                "SELECT value FROM settings WHERE key='creative_rule_summary_v1_backup'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(backup.contains("CUSTOM RESEARCH"));
        migrate(&db, Path::new("/unavailable")).unwrap();
        assert_eq!(
            super::super::storage::read(&db, "", "creative-concepts", "SKILL.md", 0, true).unwrap(),
            current
        );
    }
    #[test]
    fn preserves_authored_entry_while_replacing_its_retired_link_with_rules() {
        let db = db();
        db.execute("INSERT INTO skill_resources(skill_id,path,text) VALUES('creative-concepts','SKILL.md','CUSTOM PREFIX [research](references/research.md) CUSTOM SUFFIX')", []).unwrap();
        migrate(&db, &root()).unwrap();
        let page =
            super::super::storage::read(&db, "", "creative-concepts", "SKILL.md", 0, true).unwrap();
        let text = page["text"].as_str().unwrap();
        assert!(text.starts_with("CUSTOM PREFIX") && text.contains("CUSTOM SUFFIX"));
        assert!(text.contains("## Use evidence to change a creative decision"));
        assert!(!text.contains("references/research.md"));
    }
    #[test]
    fn fresh_and_pre_summary_upgrades_do_not_require_retired_bundle_files() {
        let db = db();
        super::super::storage::seed(&db, &root()).unwrap();
        super::super::catalog::migrate(&db, &root()).unwrap();
        super::super::concept_split::migrate(&db, &root()).unwrap();
        super::super::isolation::migrate(&db, &root()).unwrap();
        migrate(&db, &root()).unwrap();
        migrate_methods(&db, &root()).unwrap();
        migrate_video_motion(&db, &root()).unwrap();
        migrate_image_storyboard(&db, &root()).unwrap();
        migrate_image_cases(&db, &root()).unwrap();
        let count: i64 = db
            .query_row("SELECT count(*) FROM skill_resources", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 60);
        assert_eq!(db.query_row("SELECT count(*) FROM skill_resources WHERE path IN ('references/research.md','references/sources.md')", [], |r| r.get::<_, i64>(0)).unwrap(), 0);
    }
    #[test]
    fn later_method_update_has_its_own_marker_and_preserves_authored_methods() {
        let db = db();
        super::super::storage::seed(&db, &root()).unwrap();
        migrate(&db, &root()).unwrap();
        db.execute("UPDATE skill_resources SET text='CUSTOM ANIMATION METHOD' WHERE skill_id='creative-ad-director' AND path='references/animation-principles.md'", []).unwrap();
        migrate_methods(&db, &root()).unwrap();
        let text: String = db.query_row("SELECT text FROM skill_resources WHERE skill_id='creative-ad-director' AND path='references/animation-principles.md'", [], |r| r.get(0)).unwrap();
        assert_eq!(text, "CUSTOM ANIMATION METHOD");
        let backup: String = db
            .query_row(
                "SELECT value FROM settings WHERE key='creative_method_rules_v1_backup'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert!(backup.contains("CUSTOM ANIMATION METHOD"));
        migrate_methods(&db, Path::new("/unavailable")).unwrap();
    }
}
