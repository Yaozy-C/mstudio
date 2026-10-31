use super::*;
use crate::assistant::{harness::read_tools, model_profile, profiles, prompts};

#[test]
fn actual_read_tools_and_current_role_guidance_agree_for_every_builtin() {
    for role in profiles::builtins() {
        profiles::validate(&role).unwrap();
        let advertised: Vec<_> = read_tools::definitions()
            .into_iter()
            .filter(|tool| {
                matches!(
                    tool.name.as_str(),
                    "mstudio_read_materials" | "mstudio_read_assets"
                )
            })
            .filter(|tool| {
                profiles::allows(&role, "inspect")
                    && permissions::allows_resource_reader(&role, tool.name.as_str())
            })
            .map(|tool| tool.name.to_string())
            .collect();
        let mut expected = readers(&role.tool_ids);
        expected.sort_unstable();
        let mut advertised = advertised;
        advertised.sort_unstable();
        assert_eq!(advertised, expected, "{}", role.id);
        let system = prompts::system(&json!({"agent":model_profile::role(&role),"skills":[]}));
        if role.id == "video-analyst" {
            assert!(!system.contains("Current resource tools"));
            assert!(system.contains("You have no tools"));
        } else {
            assert!(
                system.contains(&format!(
                    "Current resource tools: {}",
                    json!(readers(&role.tool_ids))
                )),
                "{}",
                role.id
            );
            assert!(system.contains("same file and ID may appear in both libraries"));
            assert!(system.contains(
                "Material collection, asset confirmation and generation approval are separate"
            ));
        }
        for stale in [
            "Never associate an ordinary material ID",
            "Candidate images require user collection",
            "Never pass ordinary research materials",
        ] {
            assert!(!role.instructions.contains(stale), "{}: {stale}", role.id);
        }
    }
}

#[test]
fn resource_guidance_tracks_assigned_tools_instead_of_role_identity() {
    let mut role = profiles::builtins()
        .into_iter()
        .find(|p| p.id == "director")
        .unwrap();
    role.tool_ids.push("material-read".into());
    assert_eq!(
        readers(&role.tool_ids),
        vec!["mstudio_read_materials", "mstudio_read_assets"]
    );
    let system = prompts::system(&json!({"agent":model_profile::role(&role),"skills":[]}));
    assert!(
        system.contains(
            "Current resource tools: [\"mstudio_read_materials\",\"mstudio_read_assets\"]"
        )
    );
    role.tool_ids.retain(|id| id != "material-read");
    let updated = guidance(&model_profile::role(&role));
    assert!(updated.starts_with("Current resource tools: [\"mstudio_read_assets\"]"));
}

#[test]
fn dual_membership_handoffs_suggest_the_recipients_reader_without_reclassifying_sources() {
    let doc = json!({"assets":[
        {"id":"both","kind":"image","inMaterials":true,"assetStatus":"confirmed"},
        {"id":"source","kind":"image","inMaterials":true}
    ],"nodes":[]});
    let original = json!([
        {"reference":{"kind":"asset","id":"both"},"readWith":{"tool":"mstudio_read_assets"}},
        {"reference":{"kind":"asset","id":"source"},"readWith":{"tool":"mstudio_read_assets"}}
    ]);
    for role in profiles::builtins()
        .into_iter()
        .filter(|p| p.id != "video-analyst")
    {
        let result = manifest(&doc, &role, &original);
        assert_eq!(result[0]["reference"]["id"], "both");
        assert_eq!(
            result[0]["resourceState"],
            json!({"inMaterials":true,"assetStatus":"confirmed"})
        );
        for source in result.as_array().unwrap() {
            if let Some(tool) = source["readWith"]["tool"].as_str() {
                if tool == "mstudio_read_image" {
                    assert!(profiles::allows(&role, "read_media"));
                } else {
                    assert!(
                        readers(&role.tool_ids).contains(&tool),
                        "{}: {tool}",
                        role.id
                    );
                }
            }
        }
        match role.id.as_str() {
            "concept" | "writer" => {
                assert_eq!(result[0]["readWith"]["tool"], "mstudio_read_materials")
            }
            _ => assert_eq!(result[0]["readWith"]["tool"], "mstudio_read_assets"),
        }
        if role.id == "assets" {
            assert_eq!(result[1]["readWith"]["tool"], "mstudio_read_materials");
        }
        if role.id == "coordinator" {
            assert!(result[1].get("readWith").is_none());
            assert_eq!(result[1]["reference"]["id"], "source");
        }
    }
    assert!(original[0].get("resourceState").is_none());
}
