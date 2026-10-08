use crate::{database::Store, models::media};
use anyhow::Result;
use serde_json::{Value, json};

pub fn catalog(store: &Store, args: &Value) -> Result<Value> {
    let models: Vec<_> = media::resolved(&store.db.lock().unwrap())?
        .into_iter()
        .filter(|m| m.enabled)
        .collect();
    if let Some(id) = args["mediaModelId"].as_str() {
        let model = models
            .iter()
            .find(|m| m.id == id)
            .ok_or_else(|| anyhow::anyhow!("Media model missing or disabled"))?;
        return crate::model_adapters::prompt_rules::guidance(&store.db.lock().unwrap(), model);
    }
    let offset = args["offset"].as_u64().unwrap_or(0).min(100) as usize;
    let entries: Vec<_> = models.iter().skip(offset).take(20)
        .map(|m| json!({"id":m.id,"name":m.name,"kind":m.kind,"provider":m.plugin,"endpoint":m.endpoint})).collect();
    Ok(
        json!({"models":entries,"nextOffset":if offset+entries.len()<models.len(){Some(offset+entries.len())}else{None},
        "usage":"Select by output type. edit operations request_generation creates a generation task in the conversation; mediaKind specifies output type. Preserve the user-selected model. Submission is not completion. Read selected-model rules with mediaModelId before prompt writing unless the same full rules are already injected. The adapter validates inputs; do not guess model capabilities."}),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn shared_catalog_excludes_disabled_and_rejects_invalid_selection() {
        let root =
            std::env::temp_dir().join(format!("mstudio-media-tools-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        let models = json!([
            {"id":"image","name":"Image","kind":"image","plugin":"fal","endpoint":"fal-ai/flux/schnell","params":{},"enabled":true},
            {"id":"video","name":"Video","kind":"video","plugin":"fal","endpoint":"fal-ai/video","params":{},"enabled":false}
        ]);
        store
            .set_setting("media-models", &models.to_string())
            .unwrap();
        let result = catalog(&store, &json!({})).unwrap();
        assert_eq!(result["models"].as_array().unwrap().len(), 1);
        assert_eq!(result["models"][0]["id"], "image");
        let profiles = super::super::profiles::builtins();
        let production = profiles.iter().find(|p| p.id == "production").unwrap();
        // The retired reviewer has no successor; keep the read-only boundary with an
        // inline profile that has no media-generation tool.
        let mut read_only = profiles.iter().find(|p| p.id == "editor").unwrap().clone();
        read_only.tool_ids = vec!["project-read".into(), "memory-read".into()];
        assert!(super::super::profiles::allows(production, "models"));
        assert!(!super::super::profiles::allows(&read_only, "models"));
        let profile = serde_json::to_value(production).unwrap();
        assert!(profile.get("modelId").is_none());
        assert!(profile.get("mediaModelIds").is_none());
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
