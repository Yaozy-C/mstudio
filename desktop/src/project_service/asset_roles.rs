use serde_json::Value;

fn items(value: &Value) -> impl Iterator<Item = &Value> {
    value.as_array().into_iter().flatten()
}

pub(crate) const MATERIAL_REFERENCE_ERROR: &str =
    "此资源尚未确认为镜头资产。请先在资产库确认，再用于镜头制作；原素材会保留。";

pub(crate) fn in_library(asset: &Value) -> bool {
    asset["annotationSourceId"]
        .as_str()
        .is_none_or(str::is_empty)
        && asset["inLibrary"]
            .as_bool()
            .unwrap_or(asset["generated"] != true && asset["attachmentOnly"] != true)
}

pub(crate) fn is_asset(asset: &Value) -> bool {
    asset["assetStatus"].as_str().is_some() || asset["usage"] == "asset"
}

pub(crate) fn is_confirmed(asset: &Value) -> bool {
    is_asset(asset)
        && asset["assetStatus"]
            .as_str()
            .map_or_else(|| in_library(asset), |status| status == "confirmed")
}

pub(crate) fn is_material(document: &Value, asset: &Value) -> bool {
    asset["annotationSourceId"]
        .as_str()
        .is_none_or(str::is_empty)
        && asset["inMaterials"].as_bool().unwrap_or_else(|| {
            in_library(asset)
                && (!is_asset(asset)
                    || (asset["generated"] != true && asset["attachmentOnly"] != true))
                && (is_asset(asset) || !is_production_output(document, asset))
        })
}

pub(crate) fn is_shot_asset(asset: &Value) -> bool {
    is_confirmed(asset)
        && asset["inLibrary"] != false
        && asset["missing"] != true
        && asset["annotationSourceId"]
            .as_str()
            .is_none_or(str::is_empty)
        && matches!(asset["kind"].as_str(), Some("image" | "video" | "audio"))
}

pub(crate) fn is_production_reference(document: &Value, asset: &Value) -> bool {
    if is_shot_asset(asset) {
        return true;
    }
    if is_asset(asset) {
        return false;
    }
    is_production_output(document, asset)
}

pub(crate) fn is_production_output(document: &Value, asset: &Value) -> bool {
    let id = &asset["id"];
    items(&document["previsAssets"]).any(|a| &a["id"] == id)
        || items(&document["nodes"]).any(|n| {
            n["shot"].is_object()
                && (&n["resultAssetId"] == id
                    || items(&n["shot"]["frames"]).any(|f| &f["assetId"] == id)
                    || items(&n["shot"]["takes"]).any(|t| &t["assetId"] == id))
        })
        || document["production"]["drafts"]
            .as_object()
            .into_iter()
            .flat_map(|v| v.values())
            .any(|t| {
                (t["ownerId"].is_string() || t["targetNodeId"].is_string())
                    && t["generationPurpose"] != "asset"
                    && (&t["resultAssetId"] == id || items(&t["resultAssetIds"]).any(|v| v == id))
            })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn research_membership_and_confirmation_are_independent() {
        let doc = json!({"nodes":[]});
        let imported = json!({"id":"source","kind":"image","usage":"asset"});
        assert!(is_material(&doc, &imported));
        assert!(is_shot_asset(&imported));
        let candidate = json!({"id":"candidate","kind":"image","usage":"asset","assetStatus":"candidate","generated":true,"inLibrary":true,"inMaterials":true});
        assert!(is_material(&doc, &candidate));
        assert!(!is_shot_asset(&candidate));
        let mut confirmed = candidate.clone();
        confirmed["assetStatus"] = json!("confirmed");
        assert!(is_material(&doc, &confirmed));
        assert!(is_shot_asset(&confirmed));
        confirmed["inMaterials"] = json!(false);
        assert!(!is_material(&doc, &confirmed));
        assert!(is_shot_asset(&confirmed));
    }

    #[test]
    fn materials_cannot_bypass_asset_confirmation_but_shot_outputs_remain_usable() {
        let doc = json!({"nodes":[{"shot":{"frames":[{"assetId":"frame"}],"takes":[{"assetId":"take"}]}}]});
        assert!(!is_production_reference(
            &doc,
            &json!({"id":"source","kind":"image"})
        ));
        assert!(is_production_reference(
            &doc,
            &json!({"id":"hero","kind":"image","usage":"asset"})
        ));
        assert!(!is_production_reference(
            &doc,
            &json!({"id":"hero","kind":"image","usage":"asset","generated":true})
        ));
        assert!(is_production_reference(
            &doc,
            &json!({"id":"hero","kind":"image","usage":"asset","generated":true,"inLibrary":true})
        ));
        for id in ["frame", "take"] {
            assert!(is_production_reference(
                &doc,
                &json!({"id":id,"kind":"image"})
            ));
        }
    }
}
