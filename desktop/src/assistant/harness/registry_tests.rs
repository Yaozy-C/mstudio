use super::*;
#[test]
fn project_discovery_has_no_umbrella_read_or_edit_actions() {
    assert!(!ACTIONS.iter().any(|(name, _, _)| *name == "inspect"));
    let definitions = super::super::read_tools::definitions();
    assert!(
        definitions
            .iter()
            .any(|tool| tool.name == "mstudio_read_shots")
    );
    assert!(
        !definitions
            .iter()
            .any(|tool| tool.name == "mstudio_inspect")
    );
}

#[test]
fn delegation_catalog_read_failure_never_becomes_an_empty_enum() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    assert!(
        delegation_definition(&db, "coordinator")
            .unwrap_err()
            .contains("Cannot read Agent catalog")
    );
    db.execute_batch("CREATE TABLE settings(key TEXT PRIMARY KEY,value TEXT); INSERT INTO settings VALUES('agents','invalid current JSON');").unwrap();
    assert!(
        delegation_definition(&db, "coordinator")
            .unwrap_err()
            .contains("Cannot read Agent catalog")
    );
    assert_eq!(
        db.query_row("SELECT value FROM settings WHERE key='agents'", [], |r| {
            r.get::<_, String>(0)
        })
        .unwrap(),
        "invalid current JSON"
    );
}

#[test]
fn bound_skill_tool_references_and_activity_labels_match_available_tools() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    let labels =
        std::fs::read_to_string(root.join("frontend/src/assistant/activityRows.ts")).unwrap();
    for profile in profiles::builtins()
        .into_iter()
        .filter(|p| p.id != "video-analyst")
    {
        let mut definitions: Vec<_> = super::super::operation_tools::tools(&profile)
            .into_iter()
            .map(|t| t.definition)
            .collect();
        if profiles::allows(&profile, "inspect") {
            definitions.extend(super::super::read_tools::definitions().into_iter().filter(
                |definition| {
                    crate::assistant::permissions::allows_resource_reader(
                        &profile,
                        definition.name.as_str(),
                    )
                },
            ));
            definitions.extend([
                super::super::agent_artifacts::definition(),
                super::super::generation_tool::definition(),
            ]);
        }
        definitions.extend(super::super::media_tools::definitions(&profile));
        if profiles::allows(&profile, "delegate") {
            definitions.push(super::super::delegation::definition());
            definitions.extend(super::super::delegation::control_definitions());
        }
        let mut names: std::collections::HashSet<_> =
            definitions.iter().map(|d| d.name.to_string()).collect();
        names.insert("mstudio_read_result".into());
        names.extend(
            ACTIONS
                .iter()
                .filter(|(action, _, _)| *action != "edit" && profiles::allows(&profile, action))
                .map(|(action, _, _)| format!("mstudio_{action}")),
        );
        for definition in &definitions {
            super::super::schema_definition::check(&definition.parameters).unwrap();
        }
        for name in &names {
            assert!(
                labels.contains(&format!("{}:", name.strip_prefix("mstudio_").unwrap())),
                "{} has no activity label: {name}",
                profile.id
            );
        }
        for skill in &profile.skill_ids {
            let mut directories = vec![root.join("skills").join(skill)];
            while let Some(dir) = directories.pop() {
                for entry in std::fs::read_dir(dir).unwrap() {
                    let path = entry.unwrap().path();
                    if path.is_dir() {
                        directories.push(path);
                        continue;
                    }
                    if path.extension().and_then(|x| x.to_str()) != Some("md") {
                        continue;
                    }
                    let text = std::fs::read_to_string(&path).unwrap();
                    for word in text
                        .split(|c: char| !c.is_ascii_alphanumeric() && c != '_')
                        .filter(|w| w.starts_with("mstudio_"))
                    {
                        assert!(
                            names.contains(word),
                            "{} cannot use {word} referenced by {}",
                            profile.id,
                            path.display()
                        );
                    }
                }
            }
        }
    }
}
