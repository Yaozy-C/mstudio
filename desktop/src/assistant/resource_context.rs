//! Shared resource meaning and tool-aware guidance for current and delegated turns.
use super::{permissions, profiles::AgentProfile};
use crate::project_service::asset_roles;
use serde_json::{Value, json};

pub const POLICY: &str = "Workspace resource rules take precedence over conflicting role or Skill wording. Materials are source evidence for creative work and writing; assets are reusable production identities and references. These are independent memberships: the same file and ID may appear in both libraries. Establishing or confirming an asset preserves source material; removing one membership does not remove the other. Material collection, asset confirmation and generation approval are separate user actions. Read inMaterials and assetStatus; usage is legacy compatibility. A resource with assetStatus=confirmed remains a valid shot reference even if inMaterials=true or a reader labels it as material. A material without confirmed asset membership and an unconfirmed candidate cannot supply a reusable shot identity. An existing confirmed original does not need a generated replacement. Draft explorations remain on the draft canvas; only explicit user selection establishes a confirmed asset or adopts an output as a shot frame/take. Existing shot frames, video takes and private previs remain valid production controls under their owning shots; do not automatically classify them as research or reusable assets. Asset canvas nodes are display cards, not proof of approval. Counts report independent collections and can overlap; empty materials do not imply empty assets. A directory entry, asset approval or generation prompt does not prove actual visual content or product facts. Use real IDs with available media tools for needed pixels; a local path or another @ attachment is not a prerequisite for reading a saved image or video frame. Report concrete failed reads or missing evidence instead of claiming unreadability from absent paths. Only use tools advertised in the current turn; tool assignment, not a fixed role-name prohibition, determines resource discovery. Skills describe professional methods and do not add tools or change collection membership.";

pub fn readers(tools: &[String]) -> Vec<&'static str> {
    if !tools.iter().any(|id| id == "project-read") {
        return vec![];
    }
    ["mstudio_read_materials", "mstudio_read_assets"]
        .into_iter()
        .filter(|name| permissions::resource_reader_tools(tools, name))
        .collect()
}

pub fn guidance(agent: &Value) -> String {
    let tools: Vec<String> = serde_json::from_value(agent["tools"].clone()).unwrap_or_default();
    let readers = readers(&tools);
    let media =
        tools.iter().any(|id| id == "project-read") && tools.iter().any(|id| id == "media-read");
    format!(
        "Current resource tools: {}. Use the assigned material reader for source evidence and the assigned asset reader for confirmed references and known production outputs. Do not request a reader absent from this list. {} Resource references may be forwarded without changing their membership. If required information needs a reader you do not have, report the specific gap or hand it to the appropriately equipped specialist; do not move files, generate replacements or infer that a resource does not exist.",
        json!(readers),
        if media {
            "Media-content tools are available for needed image pixels or video frames by assetId."
        } else {
            "No active media-content reader is assigned; preserve source references for a specialist without claiming personal visual verification."
        }
    )
}

/// Adjust handoff suggestions to the recipient's actual tools; never gate references.
pub fn manifest(doc: &Value, role: &AgentProfile, original: &Value) -> Value {
    let mut manifest = original.clone();
    let available = readers(&role.tool_ids);
    for source in manifest.as_array_mut().into_iter().flatten() {
        if source["reference"]["kind"] != "asset" {
            continue;
        }
        let Some(id) = source["reference"]["id"].as_str().map(str::to_owned) else {
            continue;
        };
        let Some(asset) = crate::previs::media::find(doc, &id) else {
            continue;
        };
        let material = asset_roles::is_material(doc, asset);
        let status = if asset_roles::is_asset(asset) {
            json!(if asset_roles::is_confirmed(asset) {
                "confirmed"
            } else {
                "candidate"
            })
        } else {
            Value::Null
        };
        source["resourceState"] = json!({"inMaterials":material,"assetStatus":status});
        let reader =
            if available.contains(&"mstudio_read_assets") && asset_roles::is_confirmed(asset) {
                Some("mstudio_read_assets")
            } else if material && available.contains(&"mstudio_read_materials") {
                Some("mstudio_read_materials")
            } else if available.contains(&"mstudio_read_assets") && !material {
                Some("mstudio_read_assets")
            } else {
                None
            };
        source.as_object_mut().unwrap().remove("readWith");
        if let Some(reader) = reader {
            source["readWith"] = json!({"tool":reader,"arguments":{"ids":[id]}});
        } else if super::profiles::allows(role, "read_media")
            && matches!(asset["kind"].as_str(), Some("image" | "video"))
        {
            source["readWith"] = json!({"tool":"mstudio_read_image","arguments":{"assetId":id}});
        } else {
            source["readerNote"] = json!(
                "Source reference preserved; no matching resource reader is assigned to this Agent."
            );
        }
    }
    manifest
}

#[cfg(test)]
#[path = "resource_context_tests.rs"]
mod tests;
